use egui::Ui;

use crate::{
    app::{TimeVariable, Unit},
    App,
};

pub fn create_param_grid(app: &mut App, ui: &mut Ui) {
    egui::Grid::new("time_grid")
        .spacing([50.0, 15.0])
        .show(ui, |ui| {
            // Make the DragValue widgets a bit wider:
            ui.spacing_mut().interact_size.x = 50.0;

            let (value, unit) = app.start_time_params();
            add_time_param(
                ui,
                "Start Time: ",
                value,
                unit,
                "The time in the song where the loop will start.",
            );

            let (value, unit) = app.end_time_params();
            add_time_param(
                ui,
                "End Time: ",
                value,
                unit,
                "The time in the song where the loop will end.",
            );

            let (value, unit) = app.crossfade_params();
            add_time_param(
                ui,
                "Crossfade Duration: ",
                value,
                unit,
                "The time it takes for the loop to fade in and out.",
            );

            ui.label("Loop Count")
                .on_hover_text("The amount of times the section should loop");
            ui.add(egui::DragValue::new(app.get_loop_count()).speed(1))
                .on_hover_text("The amount of times the section should loop");
            ui.label(format!(
                "Total approximate loop time: {} seconds",
                f32::max(
                    0.0,
                    (app.get_time_var_s(TimeVariable::End)
                        - app.get_time_var_s(TimeVariable::Start))
                        * f32::from(*app.get_loop_count())
                )
            ));
            ui.end_row();

            ui.horizontal(|ui| {
                let (can_run, reason) = match app.can_loop() {
                    Ok(_) => (true, "".to_string()),
                    Err(e) => (false, e),
                };
                if ui
                    .add_enabled(can_run, egui::Button::new("Create Loop"))
                    .on_disabled_hover_text(&reason)
                    .on_hover_text("Create a loop from the provided file.")
                    .clicked()
                {
                    app.open_file_dialog_and_create_loop("output", false);
                }
                if ui
                    .add_enabled(can_run, egui::Button::new("Test Loop"))
                    .on_disabled_hover_text(&reason)
                    .on_hover_text("Test the loop with the provided file.")
                    .clicked()
                {
                    app.open_file_dialog_and_create_loop("test", true);
                }
                if app.is_running() {
                    ui.add(egui::widgets::Spinner::new());
                }
                if app.has_succeeded_running() {
                    ui.monospace(
                        egui::RichText::new("Done!").color(egui::Color32::from_rgb(100, 255, 100)),
                    );
                }
            });
        });
}

fn add_time_param<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    unit: &mut Unit,
    tooltip: &str,
) {
    ui.label(label).on_hover_text(tooltip);
    ui.add(egui::DragValue::new(value).speed(5))
        .on_hover_text(tooltip);
    ui.horizontal(|ui| {
        ui.selectable_value(unit, Unit::Milliseconds, "ms");
        ui.selectable_value(unit, Unit::Seconds, "s");
    });
    ui.end_row();
}
