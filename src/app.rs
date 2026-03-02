use std::{path::PathBuf, slice::Iter, sync::mpsc::Receiver};

use crate::{
    looper,
    ui::{
        console::create_console_view,
        error::error_window,
        ffmpeg::{ffmpeg_info, initial_ffmpeg_info},
        footer::add_footer,
        header::add_header,
        parameters::create_param_grid,
    },
};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Default)]
pub enum Unit {
    #[default]
    Milliseconds,
    Seconds,
}

#[derive(Debug)]
pub enum ConsoleText {
    Program(String),
    Success(String),
    Stdout(String),
    Stderr(String),
}

pub enum TimeVariable {
    Start,
    End,
    Crossfade,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct AppUnits {
    start_unit: Unit,
    end_unit: Unit,
    crossfade_unit: Unit,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct AppToolPaths {
    ffmpeg_path: String,
}

#[derive(Default)]
struct AppTimes {
    start_time: u32,
    end_time: u32,
    crossfade_duration: u16,
    loop_count: u8,
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
    tools: AppToolPaths,
    units: AppUnits,

    #[serde(skip)]
    file: egui::DroppedFile,
    #[serde(skip)]
    times: AppTimes,

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
    success: bool,
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

    pub fn ffmpeg_button_functionality(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Err(e) = std::process::Command::new(&path).output() {
                self.error.message = format!("Failed to run FFMPEG: {}", e);
                self.error.window = true;
            } else {
                self.tools.ffmpeg_path = path.display().to_string();
            }
        }
    }

    pub fn can_loop(&self) -> Result<(), String> {
        if self.running {
            return Err("A loop is already running.".to_string());
        }
        if self.tools.ffmpeg_path.is_empty() {
            return Err("Please provide the path to the FFMPEG executable.".to_string());
        }
        if self.file.path.is_none() {
            return Err("Please provide a file to loop.".to_string());
        }

        let start = self.get_time_var_ms(TimeVariable::Start);
        let end = self.get_time_var_ms(TimeVariable::End);
        let crossfade = self.get_time_var_ms(TimeVariable::Crossfade);
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

    pub fn open_file_dialog_and_create_loop(&mut self, file_name: &str, test_loop: bool) {
        self.console.clear();
        self.success = false;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV File", &["wav"])
            .add_filter("MP3 File", &["mp3"])
            .set_file_name(file_name)
            .set_directory(std::env::current_dir().unwrap())
            .save_file()
        {
            self.running = true;
            let (tx, rx) = std::sync::mpsc::channel();
            let (tx_finish, rx_finish) = std::sync::mpsc::channel();
            self.channels.running_rx = Some(rx);
            self.channels.running_finished = Some(rx_finish);
            looper::create_loop(
                self.get_time_var_s(TimeVariable::Start),
                self.get_time_var_s(TimeVariable::End),
                self.get_time_var_s(TimeVariable::Crossfade),
                self.times.loop_count,
                self.tools.ffmpeg_path.clone(),
                self.file.path.clone().unwrap().display().to_string(),
                path.display().to_string(),
                tx,
                tx_finish,
                test_loop,
            );
        }
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

    pub fn start_time_params(&mut self) -> (&mut u32, &mut Unit) {
        (&mut self.times.start_time, &mut self.units.start_unit)
    }

    pub fn end_time_params(&mut self) -> (&mut u32, &mut Unit) {
        (&mut self.times.end_time, &mut self.units.end_unit)
    }

    pub fn crossfade_params(&mut self) -> (&mut u16, &mut Unit) {
        (
            &mut self.times.crossfade_duration,
            &mut self.units.crossfade_unit,
        )
    }

    pub fn get_loop_count(&mut self) -> &mut u8 {
        return &mut self.times.loop_count;
    }

    pub fn clear_console(&mut self) {
        self.console.clear();
    }

    pub fn iter_console(&self) -> Iter<'_, ConsoleText> {
        return self.console.iter();
    }

    pub fn set_ffmpeg_channel(&mut self, rx: Receiver<Result<PathBuf, String>>) {
        self.channels.ffmpeg_rx = Some(rx)
    }

    pub fn set_ffmpeg_loading(&mut self, load: bool) {
        self.tool_state.ffmpeg_loading = load;
    }

    pub fn is_ffmpeg_loading(&self) -> bool {
        return self.tool_state.ffmpeg_loading;
    }

    pub fn get_ffmpeg_path(&self) -> String {
        return self.tools.ffmpeg_path.clone();
    }

    pub fn is_running(&self) -> bool {
        return self.running;
    }

    pub fn has_succeeded_running(&self) -> bool {
        return self.success;
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

        // Handle inputs and channels
        self.handle_inputs(ctx);

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
                    self.success = true;
                }
            },
            |_| {},
            &mut false,
            true,
        );

        // Window popup for errors
        error_window(ctx, &mut self.error.window, self.error.message.clone());

        // Actual view
        add_header(ctx);

        // The central panel the region left after adding TopPanel's and SidePanel's
        egui::CentralPanel::default().show(ctx, |ui| {
            // Initial state if no ffmpeg path is provided
            if self.tools.ffmpeg_path.is_empty() {
                if !self.tool_state.ffmpeg_path_check {
                    self.tool_state.ffmpeg_path_check = true;
                    if std::process::Command::new("ffmpeg").output().is_ok() {
                        self.tools.ffmpeg_path = "ffmpeg".to_string();
                    }
                }
                initial_ffmpeg_info(self, ui);
                return;
            };

            ffmpeg_info(self, ui);

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Drag and drop a file to get started.\nYou should only use .wav or .mp3 files.\nThe file name will be displayed below.");
                if self.file_load {
                    ui.add(egui::widgets::Spinner::new());
                }
            });
            ui.label(self.file.path.clone().unwrap_or_default().display().to_string());

            ui.separator();

            create_param_grid(self, ui);

            ui.separator();

            create_console_view(self, ctx, ui, new_line);
        });

        add_footer(ctx);
    }
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
