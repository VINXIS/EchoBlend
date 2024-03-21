use std::vec;
use std::io::Write;

use crate::{app, ffmpeg};

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
    is_test: bool
) {
    let thread_finished = tx_finished.clone();
    std::thread::spawn(move || {
        tx.send(Ok(app::ConsoleText::Program("Rendering intro...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", &file_path,
                "-t", &(end_s - crossfade_s).to_string(),
                "intro.wav"
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        tx.send(Ok(app::ConsoleText::Program("Rendering crossfade sample 1...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", &file_path,
                "-ss", &format!("{}", end_s - crossfade_s),
                "-t", &crossfade_s.to_string(),
                "-af", &format!("afade=t=out:st={}:d={}", end_s - crossfade_s, crossfade_s),
                "crossfade1.wav",
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }
        tx.send(Ok(app::ConsoleText::Program("Rendering crossfade sample 2...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", &file_path,
                "-ss", &format!("{}", start_s - crossfade_s),
                "-t", &crossfade_s.to_string(),
                "-af", &format!("afade=t=in:st={}:d={}", start_s - crossfade_s, crossfade_s),
                "crossfade2.wav",
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }
        tx.send(Ok(app::ConsoleText::Program("Rendering crossfade...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", "crossfade1.wav",
                "-i", "crossfade2.wav",
                "-filter_complex", "amix=inputs=2:duration=first:dropout_transition=0:normalize=0",
                "crossfade.wav",
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        tx.send(Ok(app::ConsoleText::Program("Deleting crossfade samples...".to_string()))).unwrap();
        std::fs::remove_file("crossfade1.wav").unwrap();
        std::fs::remove_file("crossfade2.wav").unwrap();

        tx.send(Ok(app::ConsoleText::Program("Rendering loop segment...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", &file_path,
                "-ss", &start_s.to_string(),
                "-t", (end_s - start_s - crossfade_s).to_string().as_str(),
                "loop.wav",
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        tx.send(Ok(app::ConsoleText::Program("Rendering outro...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &[
                "-y",
                "-i", &file_path,
                "-ss", &start_s.to_string(),
                "outro.wav",
            ],
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        let mut cmd = vec!["-y"];
        if is_test {
            cmd.append(&mut vec![
                "-i", "intro.wav",
                "-i", "crossfade.wav",
                "-i", "loop.wav",
                "-i", "crossfade.wav",
                "-i", "outro.wav",
                "-filter_complex", "concat=n=5:v=0:a=1",
                &output_path,
            ]);
        } else {
            // Create an ffmpeg concat list txt file
            let mut concat_list = std::fs::File::create("concat_list.txt").unwrap();
            concat_list.write_all(b"file 'intro.wav'\n").unwrap();
            for _ in 0..loop_count {
                concat_list.write_all(b"file 'crossfade.wav'\n").unwrap();
                concat_list.write_all(b"file 'loop.wav'\n").unwrap();
            }
            concat_list.write_all(b"file 'crossfade.wav'\n").unwrap();
            concat_list.write_all(b"file 'outro.wav'\n").unwrap();

            cmd.append(&mut vec![
                "-f", "concat",
                "-safe", "0",
                "-i", "concat_list.txt",
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
        tx.send(Ok(app::ConsoleText::Program("Merging segments...".to_string()))).unwrap();
        if execute_ffmpeg_command(
            &ffmpeg_path,
            &cmd,
            &tx
        ).is_err() {
            thread_finished.send(Ok(true)).unwrap();
            return;
        }

        tx.send(Ok(app::ConsoleText::Program("Deleting segments...".to_string()))).unwrap();
        std::fs::remove_file("concat_list.txt").unwrap_or_default();
        std::fs::remove_file("intro.wav").unwrap();
        std::fs::remove_file("crossfade.wav").unwrap();
        std::fs::remove_file("loop.wav").unwrap();
        std::fs::remove_file("outro.wav").unwrap();

        tx.send(Ok(app::ConsoleText::Program("Done!".to_string()))).unwrap();
        tx_finished.send(Ok(true)).unwrap();
    });
}

fn execute_ffmpeg_command(
    ffmpeg_path: &str,
    args: &[&str],
    tx: &std::sync::mpsc::Sender<Result<app::ConsoleText, String>>
) -> Result<(), ()> {
    match ffmpeg::run_ffmpeg(ffmpeg_path, args, tx) {
        Ok(_) => Ok(()),
        Err(e) => {
            tx.send(Err(e)).unwrap();
            Err(())
        }
    }
}