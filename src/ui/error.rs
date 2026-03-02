pub fn error_window(ctx: &egui::Context, show_window: &mut bool, err: String) {
    if *show_window {
        egui::Window::new("Error")
            .open(show_window)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(err);
            });
    }
}
