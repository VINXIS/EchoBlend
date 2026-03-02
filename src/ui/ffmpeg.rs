use egui::Ui;

use crate::{ffmpeg, App};

pub fn initial_ffmpeg_info(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            "This program requires FFMPEG.\nPlease provide the path to the FFMPEG executable.",
        );
        if ui.button("Browse for ffmpeg.exe").clicked() {
            app.ffmpeg_button_functionality();
        } else if std::env::consts::OS == "windows" && ui.button("Download FFMPEG").clicked() {
            app.set_ffmpeg_loading(true);
            let (tx, rx) = std::sync::mpsc::channel();
            ffmpeg::get_ffmpeg(tx);
            app.set_ffmpeg_channel(rx);
        }
        if app.is_ffmpeg_loading() {
            ui.add(egui::widgets::Spinner::new());
        }
    });
}

pub fn ffmpeg_info(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button("Change FFMPEG Path").clicked() {
            app.ffmpeg_button_functionality();
        }
        ui.label(format!("FFMPEG Path: {}", app.get_ffmpeg_path()));
    });
}
