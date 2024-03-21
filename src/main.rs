#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 520.0])
            .with_min_inner_size([430.0, 300.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../icon.png")[..]).unwrap(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Echo Blend",
        native_options,
        Box::new(|cc| Box::new(echo_blend::App::new(cc))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    println!("Ensure you access the webpage at http://127.0.0.1:8080/index.html#dev");

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(echo_blend::App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
