use egui::Context;

pub fn add_header(ctx: &Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::warn_if_debug_build(ui);
        ui.heading(egui::RichText::new("Echo Blend").text_style(egui::TextStyle::Heading).size(24.0));
        ui.label(egui::RichText::new("This is an app that allows you to loop a section of a song for an extended period of time.").size(16.0));
        egui::widgets::global_dark_light_mode_buttons(ui);
    });
}
