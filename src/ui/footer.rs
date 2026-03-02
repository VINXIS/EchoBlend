use egui::Context;

pub fn add_footer(ctx: &Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.add(egui::github_link_file!(
            "https://github.com/VINXIS/echoblend/",
            "Source Code | Created by VINXIS"
        ));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Powered by ");
            ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            ui.label(" and ");
            ui.hyperlink_to(
                "eframe",
                "https://github.com/emilk/egui/tree/master/crates/eframe",
            );
            ui.label(".");
        });
    });
}
