use crate::{ffmpeg, looper};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Default)]
pub enum Unit {
    #[default]
    Milliseconds,
    Seconds,
}

#[derive(Debug)]
pub enum ConsoleText {
    Program(String),
    Stdout(String),
    Stderr(String),
}

pub enum TimeVariable {
    Start,
    End,
    Crossfade,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AppUnits {
    pub start_unit: Unit,
    pub end_unit: Unit,
    pub crossfade_unit: Unit,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AppToolPaths {
    pub ffmpeg_path: String,
}

#[derive(Default)]
pub struct AppTimes {
    pub start_time: u32,
    pub end_time: u32,
    pub crossfade_duration: u16,
    pub loop_count: u8,
}

#[derive(Default)]
struct AppToolState {
    ffmpeg_path_check: bool,
    ffmpeg_loading: bool,
}

#[derive(Default)]
struct AppError {
    message: String,
    window: bool,
}

#[derive(Default)]
struct AppChannels {
    ffmpeg_rx: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    running_rx: Option<std::sync::mpsc::Receiver<Result<ConsoleText, String>>>,
    running_finished: Option<std::sync::mpsc::Receiver<Result<bool, String>>>,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct App {
    pub tools: AppToolPaths,
    pub units: AppUnits,

    #[serde(skip)]
    pub file: egui::DroppedFile,
    #[serde(skip)]
    pub times: AppTimes,

    #[serde(skip)]
    error: AppError,
    #[serde(skip)]
    tool_state: AppToolState,
    #[serde(skip)]
    channels: AppChannels,

    #[serde(skip)]
    file_load: bool,
    #[serde(skip)]
    running: bool,
    #[serde(skip)]
    console: Vec<ConsoleText>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn handle_inputs(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Hovered/Dropped files
            if i.raw.hovered_files.is_empty() && i.raw.dropped_files.is_empty() {
                self.file_load = false;
            } else if !i.raw.hovered_files.is_empty() {
                self.file_load = true;
            } else if !i.raw.dropped_files.is_empty() {
                let target_file = &i.raw.dropped_files[0];
                // Check if file is wav or mp3
                self.file_load = false;
                let path = match target_file.clone().path {
                    Some(p) => match p.extension() {
                        Some(ext) => ext.to_str().unwrap_or_default().to_string(),
                        None => "".to_string(),
                    },
                    None => "".to_string(),
                };
                if target_file.name.ends_with(".wav")
                    || target_file.name.ends_with(".mp3")
                    || path.ends_with("wav")
                    || path.ends_with("mp3")
                {
                    self.file = target_file.clone();
                } else {
                    self.error.message = format!(
                        "You can only use .wav or .mp3 files. Your file was: {}",
                        target_file.clone().path.expect("No Path").display()
                    );
                    self.error.window = true;
                }
            }
        });
    }

    pub fn get_time_var_ms(&self, var: TimeVariable) -> u32 {
        let ms = match var {
            TimeVariable::Start => self.times.start_time,
            TimeVariable::End => self.times.end_time,
            TimeVariable::Crossfade => self.times.crossfade_duration as u32,
        };
        ms * match var {
            TimeVariable::Start => match self.units.start_unit {
                Unit::Milliseconds => 1,
                Unit::Seconds => 1000,
            },
            TimeVariable::End => match self.units.end_unit {
                Unit::Milliseconds => 1,
                Unit::Seconds => 1000,
            },
            TimeVariable::Crossfade => match self.units.crossfade_unit {
                Unit::Milliseconds => 1,
                Unit::Seconds => 1000,
            },
        }
    }

    pub fn get_time_var_s(&self, var: TimeVariable) -> f32 {
        self.get_time_var_ms(var) as f32 / 1000.0
    }
}

impl eframe::App for App {
    // Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self);
    }

    // Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Base Style
        ctx.style_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.show_tooltips_only_when_still = false;
        });

        // Case checks
        if self.error.window {
            egui::Window::new("Error")
                .open(&mut self.error.window)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(self.error.message.clone());
                });
        }

        // Handle inputs
        self.handle_inputs(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            ui.heading(egui::RichText::new("Echo Blend").text_style(egui::TextStyle::Heading).size(24.0));
            ui.label(egui::RichText::new("This is an app that allows you to loop a section of a song for an extended period of time.").size(16.0));
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            handle_rx(
                &mut self.channels.ffmpeg_rx,
                |path| self.tools.ffmpeg_path = path.display().to_string(),
                |e| {
                    self.error.message = format!("Failed to download FFMPEG: {}", e);
                    self.error.window = true;
                },
                &mut self.tool_state.ffmpeg_loading,
                true,
            );

            let mut new_line = false;
            handle_rx(
                &mut self.channels.running_rx,
                |res| {
                    new_line = true;
                    self.console.push(res);
                },
                |e| {
                    self.error.message = e;
                    self.error.window = true;
                },
                &mut self.running,
                false,
            );

            handle_rx(
                &mut self.channels.running_finished,
                |res| {
                    if res {
                        self.channels.running_rx = None;
                        self.running = false;
                    }
                },
                |_| {},
                &mut false,
                true,
            );

            // Initial state if no ffmpeg path is provided
            if self.tools.ffmpeg_path.is_empty() {
                if !self.tool_state.ffmpeg_path_check {
                    self.tool_state.ffmpeg_path_check = true;
                    if std::process::Command::new("ffmpeg").output().is_ok() {
                        self.tools.ffmpeg_path = "ffmpeg".to_string();
                    }
                }
                initial_central_panel(self, ui);
                return;
            };

            ui.horizontal(|ui| {
                if ui.button("Change FFMPEG Path").clicked() {
                    ffmpeg_button_functionality(self);
                }
                ui.label(format!("FFMPEG Path: {}", self.tools.ffmpeg_path));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Drag and drop a file to get started.\nYou should only use .wav or .mp3 files.\nThe file name will be displayed below.");
                if self.file_load {
                    ui.add(egui::widgets::Spinner::new());
                }
            });
            ui.label(self.file.path.clone().unwrap_or_default().display().to_string());
            ui.separator();

            egui::Grid::new("time_grid")
                .spacing([50.0, 15.0])
                .show(ui, |ui| {
                    // Make the DragValue widgets a bit wider:
                    ui.spacing_mut().interact_size.x = 50.0;

                    add_time_setting(
                        ui,
                        "Start Time: ",
                        &mut self.times.start_time,
                        &mut self.units.start_unit,
                        "The time in the song where the loop will start."
                    );

                    add_time_setting(
                        ui,
                        "End Time: ",
                        &mut self.times.end_time,
                        &mut self.units.end_unit,
                        "The time in the song where the loop will end."
                    );

                    add_time_setting(
                        ui,
                        "Crossfade Duration: ",
                        &mut self.times.crossfade_duration,
                        &mut self.units.crossfade_unit,
                        "The time it takes for the loop to fade in and out."
                    );

                    ui.label("Loop Count").on_hover_text("The amount of times the section should loop");
                    ui.add(egui::DragValue::new(&mut self.times.loop_count).speed(1)).on_hover_text("The amount of times the section should loop");
                    ui.label(format!("Total approximate loop time: {} seconds", f32::max(0.0, (self.get_time_var_s(TimeVariable::End) - self.get_time_var_s(TimeVariable::Start)) * f32::from(self.times.loop_count))));
                    ui.end_row();

                    ui.horizontal(|ui| {
                        let (can_run, reason) = match can_loop(self) {
                            Ok(_) => (true, "".to_string()),
                            Err(e) => (false, e),
                        };
                        if ui.add_enabled(can_run, egui::Button::new("Create Loop"))
                            .on_disabled_hover_text(&reason)
                            .on_hover_text("Create a loop from the provided file.")
                            .clicked()
                        {
                            open_file_dialog_and_create_loop(self, "output", false);
                        }
                        if ui.add_enabled(can_run, egui::Button::new("Test Loop"))
                            .on_disabled_hover_text(&reason)
                            .on_hover_text("Test the loop with the provided file.")
                            .clicked()
                        {
                            open_file_dialog_and_create_loop(self, "test", true);
                        }
                        if ui.add(egui::Button::new("Clear Console"))
                            .on_disabled_hover_text(&reason)
                            .on_hover_text("Clear the console.")
                            .clicked()
                        {
                            self.console.clear();
                        }
                        if self.running {
                            ui.add(egui::widgets::Spinner::new());
                        }
                    });
                });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().visuals.window_fill = egui::Color32::from_rgb(0, 0, 0);
                ui.label("Console");
                ui.separator();
                if ctx.style().visuals.dark_mode {
                    for line in self.console.iter() {
                        match line {
                            ConsoleText::Program(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(255, 255, 100))),
                            ConsoleText::Stdout(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(100, 255, 100))),
                            ConsoleText::Stderr(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(255, 100, 100))),
                        };
                    }
                } else {
                    for line in self.console.iter() {
                        match line {
                            ConsoleText::Program(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(100, 100, 0))),
                            ConsoleText::Stdout(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(0, 100, 0))),
                            ConsoleText::Stderr(text) => ui.monospace(egui::RichText::new(text).color(egui::Color32::from_rgb(100, 0, 0))),
                        };
                    }
                }
                if new_line {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add(egui::github_link_file!(
                "https://github.com/VINXIS/echoblend/",
                "Source Code | Created by VINXIS"
            ));
            powered_by_egui_and_eframe(ui);
        });
    }
}

fn ffmpeg_button_functionality(app: &mut App) {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        if let Err(e) = std::process::Command::new(&path).output() {
            app.error.message = format!("Failed to run FFMPEG: {}", e);
            app.error.window = true;
        } else {
            app.tools.ffmpeg_path = path.display().to_string();
        }
    }
}

fn initial_central_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            "This program requires FFMPEG.\nPlease provide the path to the FFMPEG executable.",
        );
        if ui.button("Browse for ffmpeg.exe").clicked() {
            ffmpeg_button_functionality(app);
        } else if std::env::consts::OS == "windows" && ui.button("Download FFMPEG").clicked() {
            app.tool_state.ffmpeg_loading = true;
            let (tx, rx) = std::sync::mpsc::channel();
            ffmpeg::get_ffmpeg(tx);
            app.channels.ffmpeg_rx = Some(rx);
        }
        if app.tool_state.ffmpeg_loading {
            ui.add(egui::widgets::Spinner::new());
        }
    });
}

fn add_time_setting<T: egui::emath::Numeric>(
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

fn handle_rx<R, F, G>(
    rx_option: &mut Option<std::sync::mpsc::Receiver<Result<R, String>>>,
    on_success: F,
    on_error: G,
    loading_flag: &mut bool,
    auto_kill: bool,
) where
    F: FnOnce(R),      // Success handler
    G: FnOnce(String), // Error handler
{
    if let Some(rx) = rx_option {
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(value) => on_success(value),
                Err(e) => on_error(e),
            }
            if auto_kill {
                *rx_option = None;
                *loading_flag = false;
            }
        }
    }
}

fn can_loop(app: &mut App) -> Result<(), String> {
    if app.running {
        return Err("A loop is already running.".to_string());
    }
    if app.tools.ffmpeg_path.is_empty() {
        return Err("Please provide the path to the FFMPEG executable.".to_string());
    }
    if app.file.path.is_none() {
        return Err("Please provide a file to loop.".to_string());
    }

    let start = app.get_time_var_ms(TimeVariable::Start);
    let end = app.get_time_var_ms(TimeVariable::End);
    let crossfade = app.get_time_var_ms(TimeVariable::Crossfade);
    if start >= end {
        return Err(format!(
            "The start time must be less than the end time. Start: {}, End: {}",
            start, end
        ));
    }
    if crossfade >= start {
        return Err(format!(
            "The crossfade duration must be less than the start time. Crossfade: {}, Start: {}",
            crossfade, start
        ));
    }
    if crossfade >= end - start {
        return Err(format!("The crossfade duration must be less than the loop duration. Crossfade: {}, Loop Duration: {}", crossfade, end - start));
    }
    Ok(())
}

fn open_file_dialog_and_create_loop(app: &mut App, file_name: &str, test_loop: bool) {
    app.console.clear();
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("WAV File", &["wav"])
        .add_filter("MP3 File", &["mp3"])
        .set_file_name(file_name)
        .set_directory(std::env::current_dir().unwrap())
        .save_file()
    {
        app.running = true;
        let (tx, rx) = std::sync::mpsc::channel();
        let (tx_finish, rx_finish) = std::sync::mpsc::channel();
        app.channels.running_rx = Some(rx);
        app.channels.running_finished = Some(rx_finish);
        looper::create_loop(
            app.get_time_var_s(TimeVariable::Start),
            app.get_time_var_s(TimeVariable::End),
            app.get_time_var_s(TimeVariable::Crossfade),
            app.times.loop_count,
            app.tools.ffmpeg_path.clone(),
            app.file.path.clone().unwrap().display().to_string(),
            path.display().to_string(),
            tx,
            tx_finish,
            test_loop,
        );
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
