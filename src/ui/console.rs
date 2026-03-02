use egui::{Context, Ui};

use crate::{app::ConsoleText, App};

pub fn create_console_view(app: &mut App, ctx: &Context, ui: &mut Ui, new_line: bool) {
    egui::CollapsingHeader::new("Console")
        .default_open(true)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().visuals.window_fill = egui::Color32::from_rgb(0, 0, 0);
                if ui
                    .add(egui::Button::new("Clear Console"))
                    .on_hover_text("Clear the console.")
                    .clicked()
                {
                    app.clear_console();
                }
                ui.separator();
                let is_dark_mode = ctx.style().visuals.dark_mode;
                for line in app.iter_console() {
                    line_to_text(ui, is_dark_mode, line);
                }
                if new_line {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
        });
}

fn line_to_text(ui: &mut Ui, is_dark_mode: bool, line: &ConsoleText) {
    let mode_to_colour = |dark: (u8, u8, u8), light: (u8, u8, u8)| {
        let (r, g, b) = if is_dark_mode { dark } else { light };
        return egui::Color32::from_rgb(r, g, b);
    };

    let (text, colour) = match line {
        ConsoleText::Program(text) => (text, mode_to_colour((255, 255, 255), (0, 0, 0))),
        ConsoleText::Success(text) => (text, mode_to_colour((100, 255, 100), (0, 100, 0))),
        ConsoleText::Stdout(text) => (text, mode_to_colour((255, 255, 100), (100, 100, 0))),
        ConsoleText::Stderr(text) => (text, mode_to_colour((255, 100, 100), (100, 0, 0))),
    };

    ui.monospace(egui::RichText::new(text).color(colour));
}
