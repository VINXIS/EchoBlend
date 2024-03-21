use crate::app;
use std::io::BufRead;

const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z";

pub fn run_ffmpeg(
    ffmpeg_path: &str,
    args: &[&str],
    tx: &std::sync::mpsc::Sender<Result<app::ConsoleText, String>>,
) -> Result<(), String> {
    let mut cmd = match std::process::Command::new(ffmpeg_path)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()) // Also capture stderr
        .spawn()
    {
        Ok(cmd) => cmd,
        Err(e) => {
            let err_msg = e.to_string();
            let _ = tx.send(Err(err_msg.clone())); // Handle send error gracefully
            return Err(err_msg);
        }
    };

    // Spawn a thread to handle stdout
    let stdout = cmd.stdout.take().unwrap();
    let tx_stdout = tx.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let _ = tx_stdout.send(Ok(app::ConsoleText::Stdout(line))); // Handle send error gracefully
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = tx_stdout.send(Err(err_msg.clone())); // Handle send error gracefully
                    return;
                }
            }
        }
    });

    // Spawn another thread to handle stderr
    let stderr = cmd.stderr.take().unwrap();
    let tx_stderr = tx.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.to_lowercase().contains("error") {
                        let _ = tx_stderr.send(Ok(app::ConsoleText::Stderr(line)));
                    // Adjust as needed
                    } else {
                        let _ = tx_stderr.send(Ok(app::ConsoleText::Stdout(line)));
                        // Handle send error gracefully
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = tx_stderr.send(Err(err_msg.clone())); // Handle send error gracefully
                    return;
                }
            }
        }
    });

    // Check the process's exit status
    match cmd.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            let err_msg = format!("ffmpeg exited with error code: {}", status);
            let _ = tx.send(Err(err_msg.clone())); // Handle send error gracefully
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = tx.send(Err(err_msg.clone())); // Handle send error gracefully
            Err(err_msg)
        }
    }
}

pub fn get_ffmpeg(tx: std::sync::mpsc::Sender<Result<std::path::PathBuf, String>>) {
    std::thread::spawn(move || {
        let result = (|| -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
            let ffmpeg_path = std::env::current_dir()?
                .join("ffmpeg")
                .join("bin")
                .join("ffmpeg.exe");
            if ffmpeg_path.exists() {
                Ok(ffmpeg_path)
            } else {
                download_ffmpeg()?;
                let ffmpeg_path = extract_ffmpeg()?;
                std::process::Command::new(&ffmpeg_path).output()?;
                Ok(std::path::PathBuf::from(ffmpeg_path))
            }
        })();

        // Send the result back to the UI thread
        let _ = tx.send(result.map_err(|e| e.to_string()));
    });
}

fn download_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::blocking::get(FFMPEG_URL)?;
    let mut file = std::fs::File::create("ffmpeg.7z")?;
    std::io::copy(&mut response, &mut file)?;
    Ok(())
}

// Extract ffmpeg and provide the path to the binary
fn extract_ffmpeg() -> Result<String, Box<dyn std::error::Error>> {
    std::process::Command::new("7z")
        .arg("x")
        .arg("ffmpeg.7z")
        .arg("-o./ffmpeg")
        .arg("-y")
        .output()?;
    std::fs::remove_file("ffmpeg.7z")?;
    let dir = std::env::current_dir()?.join("ffmpeg");
    let ffmpeg_path = find_ffmpeg(&dir)?;
    Ok(ffmpeg_path)
}

fn find_ffmpeg(dir: &std::path::PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let mut ffmpeg_path = String::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "exe" {
                    ffmpeg_path = path.to_str().unwrap().to_string();
                    break;
                }
            }
        } else if path.is_dir() {
            ffmpeg_path = find_ffmpeg(&path)?;
            if !ffmpeg_path.is_empty() {
                break;
            }
        }
    }
    Ok(ffmpeg_path)
}
