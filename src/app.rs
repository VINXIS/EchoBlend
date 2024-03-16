#[derive(serde::Deserialize, serde::Serialize)]
#[derive(PartialEq)]
pub enum Unit {
    Milliseconds,
    Seconds,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    // Example stuff:
    start_time: u32,
    end_time: u32,
    crossfade_duration: u16,
    total_duration: u8,
    start_unit: Unit,
    end_unit: Unit,
    crossfade_unit: Unit,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Example stuff:
            start_time: 0,
            end_time: 0,
            crossfade_duration: 0,
            total_duration: 10,
            start_unit: Unit::Milliseconds,
            end_unit: Unit::Milliseconds,
            crossfade_unit: Unit::Milliseconds,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            ui.heading("Echo Blend");
            ui.label("This is a website that allows you to loop a section of a song for an extended period of time.");
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            if ui.button("Open file…").clicked() {
                // Logic for opening a file goes here
            }
            
            egui::Grid::new("time_grid")
                .spacing([50.0, 15.0])
                .show(ui, |ui| {
                    // Make the DragValue widgets a bit wider:
                    ui.spacing_mut().interact_size.x = 50.0;

                    ui.label("Start Time: ").on_hover_text("The time in the song where the loop will start.");
                    ui.add(egui::DragValue::new(&mut self.start_time).speed(5)).on_hover_text("The time in the song where the loop will start.");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.start_unit, Unit::Milliseconds, "ms");
                        ui.selectable_value(&mut self.start_unit, Unit::Seconds, "s");
                    });
                    ui.end_row();

                    ui.label("End Time: ").on_hover_text("The time in the song where the loop will end.");
                    ui.add(egui::DragValue::new(&mut self.end_time).speed(5)).on_hover_text("The time in the song where the loop will end.");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.end_unit, Unit::Milliseconds, "ms");
                        ui.selectable_value(&mut self.end_unit, Unit::Seconds, "s");
                    });
                    ui.end_row();

                    ui.label("Crossfade Duration: ").on_hover_text("The time it takes for the loop to fade in and out.");
                    ui.add(egui::DragValue::new(&mut self.crossfade_duration)).on_hover_text("The time it takes for the loop to fade in and out.");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.crossfade_unit, Unit::Milliseconds, "ms");
                        ui.selectable_value(&mut self.crossfade_unit, Unit::Seconds, "s");
                    });
                    ui.end_row();

                    ui.label("Total Duration (hrs): ");
                    ui.add(egui::Slider::new(&mut self.total_duration, 1..=10)).on_hover_text("The total duration of the loop.");
                });

            ui.horizontal(|ui| {
                if ui.button("Create Loop").clicked() {
                    // Calls a function to create a loop
                }
                if ui.button("Test Loop").clicked() {
                    // Calls the same function as Create Loop, but creates a loop that only lasts once
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add(egui::github_link_file!(
                "https://github.com/VINXIS/echoblend/",
                "Source Code"
            ));
            powered_by_egui_and_eframe(ui);
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
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
}
