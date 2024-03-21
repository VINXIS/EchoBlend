use std::io::Write;
use std::vec;

use crate::{app, ffmpeg};

#[allow(clippy::too_many_arguments)]
pub fn create_loop(
    start_s: f32,
    end_s: f32,
    crossfade_s: f32,
    loop_count: u8,
    ffmpeg_path: String,
    file_path: String,
    output_path: String,
    tx: std::sync::mpsc::Sender<Result<app::ConsoleText, String>>,
    tx_finished: std::sync::mpsc::Sender<Result<bool, String>>,
    is_test: bool,
) {
    let thread_finished = tx_finished.clone();
    std::thread::spawn(move || {
        // Generate unique file names to avoid conflicts
        let intro_file_name = format!("intro_echo_blend_{}.wav", std::process::id());
        let outro_file_name = format!("outro_echo_blend_{}.wav", std::process::id());
        let loop_file_name = format!("loop_echo_blend_{}.wav", std::process::id());
        let crossfade_1_file_name = format!("crossfade_echo_blend_1_{}.wav", std::process::id());
        let crossfade_2_file_name = format!("crossfade_echo_blend_2_{}.wav", std::process::id());
        let crossfade_file_name = format!("crossfade_echo_blend_{}.wav", std::process::id());
        let concat_list_file_name = format!("concat_list_echo_blend_{}.txt", std::process::id());

        if crossfade_s == 0.0 {
            tx.send(Ok(app::ConsoleText::Program(
                "Crossfade duration is 0, skipping crossfade...".to_string(),
            )))
            .unwrap();
        }

        if is_test {
            tx.send(Ok(app::ConsoleText::Program(
                "Test run, skipping loop segment...".to_string(),
            )))
            .unwrap();
        }

        tx.send(Ok(app::ConsoleText::Program(
            "Rendering intro...".to_string(),
        )))
        .unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i",
                &file_path,
                "-t",
                &(end_s - crossfade_s).to_string(),
                &intro_file_name,
            ],
            &tx,
        )
        .is_err()
        {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        if crossfade_s > 0.0 {
            tx.send(Ok(app::ConsoleText::Program(
                "Rendering crossfade sample 1...".to_string(),
            )))
            .unwrap();
            if execute_ffmpeg_command(
                &ffmpeg_path,
                &[
                    "-y",
                    "-i",
                    &file_path,
                    "-ss",
                    &format!("{}", end_s - crossfade_s),
                    "-t",
                    &crossfade_s.to_string(),
                    "-af",
                    &format!("afade=t=out:st={}:d={}", end_s - crossfade_s, crossfade_s),
                    &crossfade_1_file_name,
                ],
                &tx,
            )
            .is_err()
            {
                thread_finished.send(Ok(true)).unwrap();
                return;
            }
            tx.send(Ok(app::ConsoleText::Program(
                "Rendering crossfade sample 2...".to_string(),
            )))
            .unwrap();
            if execute_ffmpeg_command(
                &ffmpeg_path,
                &[
                    "-y",
                    "-i",
                    &file_path,
                    "-ss",
                    &format!("{}", start_s - crossfade_s),
                    "-t",
                    &crossfade_s.to_string(),
                    "-af",
                    &format!("afade=t=in:st={}:d={}", start_s - crossfade_s, crossfade_s),
                    &crossfade_2_file_name,
                ],
                &tx,
            )
            .is_err()
            {
                thread_finished.send(Ok(true)).unwrap();
                return;
            }
            tx.send(Ok(app::ConsoleText::Program(
                "Rendering crossfade...".to_string(),
            )))
            .unwrap();
            if execute_ffmpeg_command(
                &ffmpeg_path,
                &[
                    "-y",
                    "-i",
                    &crossfade_1_file_name,
                    "-i",
                    &crossfade_2_file_name,
                    "-filter_complex",
                    "amix=inputs=2:duration=first:dropout_transition=0:normalize=0",
                    &crossfade_file_name,
                ],
                &tx,
            )
            .is_err()
            {
                thread_finished.send(Ok(true)).unwrap();
                return;
            }

            tx.send(Ok(app::ConsoleText::Program(
                "Deleting crossfade samples...".to_string(),
            )))
            .unwrap();
            std::fs::remove_file(crossfade_1_file_name).unwrap();
            std::fs::remove_file(crossfade_2_file_name).unwrap();
        }

        if !is_test {
            tx.send(Ok(app::ConsoleText::Program(
                "Rendering loop segment...".to_string(),
            )))
            .unwrap();
            if execute_ffmpeg_command(
                &ffmpeg_path,
                &[
                    "-y",
                    "-i",
                    &file_path,
                    "-ss",
                    &start_s.to_string(),
                    "-t",
                    (end_s - start_s - crossfade_s).to_string().as_str(),
                    &loop_file_name,
                ],
                &tx,
            )
            .is_err()
            {
                thread_finished.send(Ok(true)).unwrap();
                return;
            }
        }

        tx.send(Ok(app::ConsoleText::Program(
            "Rendering outro...".to_string(),
        )))
        .unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i",
                &file_path,
                "-ss",
                &start_s.to_string(),
                &outro_file_name,
            ],
            &tx,
        )
        .is_err()
        {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        let mut cmd = vec!["-y"];
        if is_test {
            if crossfade_s > 0.0 {
                cmd.append(&mut vec![
                    "-i",
                    &intro_file_name,
                    "-i",
                    &crossfade_file_name,
                    "-i",
                    &outro_file_name,
                    "-filter_complex",
                    "concat=n=3:v=0:a=1",
                    &output_path,
                ]);
            } else {
                cmd.append(&mut vec![
                    "-i",
                    &intro_file_name,
                    "-i",
                    &outro_file_name,
                    "-filter_complex",
                    "concat=n=2:v=0:a=1",
                    &output_path,
                ]);
            }
        } else {
            // Create an ffmpeg concat list txt file
            let mut concat_list = std::fs::File::create(&concat_list_file_name).unwrap();
            concat_list
                .write_all(format!("file '{}'\n", &intro_file_name).as_bytes())
                .unwrap();
            for _ in 0..loop_count {
                if crossfade_s > 0.0 {
                    concat_list
                        .write_all(format!("file '{}'\n", &crossfade_file_name).as_bytes())
                        .unwrap();
                }
                concat_list
                    .write_all(format!("file '{}'\n", &loop_file_name).as_bytes())
                    .unwrap();
            }
            if crossfade_s > 0.0 {
                concat_list
                    .write_all(format!("file '{}'\n", &crossfade_file_name).as_bytes())
                    .unwrap();
            }
            concat_list
                .write_all(format!("file '{}'\n", &outro_file_name).as_bytes())
                .unwrap();

            cmd.append(&mut vec![
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                &concat_list_file_name,
            ]);
            if output_path.ends_with(".mp3") {
                cmd.push("-q:a");
                cmd.push("2");
            } else {
                cmd.push("-c");
                cmd.push("copy");
            }
            cmd.push(&output_path);
        }
        tx.send(Ok(app::ConsoleText::Program(
            "Merging segments...".to_string(),
        )))
        .unwrap();
        if execute_ffmpeg_command(&ffmpeg_path, &cmd, &tx).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        tx.send(Ok(app::ConsoleText::Program(
            "Deleting segments...".to_string(),
        )))
        .unwrap();
        std::fs::remove_file(concat_list_file_name).unwrap_or_default();
        std::fs::remove_file(intro_file_name).unwrap();
        std::fs::remove_file(outro_file_name).unwrap();
        if !is_test {
            std::fs::remove_file(loop_file_name).unwrap();
        }
        if crossfade_s > 0.0 {
            std::fs::remove_file(crossfade_file_name).unwrap();
        }

        tx.send(Ok(app::ConsoleText::Program("Done!".to_string())))
            .unwrap();
        tx_finished.send(Ok(true)).unwrap();
    });
}

fn execute_ffmpeg_command(
    ffmpeg_path: &str,
    args: &[&str],
    tx: &std::sync::mpsc::Sender<Result<app::ConsoleText, String>>,
) -> Result<(), ()> {
    match ffmpeg::run_ffmpeg(ffmpeg_path, args, tx) {
        Ok(_) => Ok(()),
        Err(e) => {
            tx.send(Err(e)).unwrap();
            Err(())
        }
    }
}
