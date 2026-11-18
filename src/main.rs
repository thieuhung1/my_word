mod app;
mod document;
mod editor;
mod icons;
mod layout;
mod platform;
mod telex;
mod ui;

use app::MyWordApp;
use eframe::egui;

fn load_app_icon() -> egui::IconData {
    let rgba = include_bytes!("../assets/icon_32.rgba").to_vec();
    egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 860.0])
            .with_min_inner_size([760.0, 540.0])
            .with_title("Rust Word Processor - Soạn thảo văn bản tiếng Việt")
            .with_icon(load_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "my_word_processor",
        options,
        Box::new(|cc| {
            platform::setup_fonts(&cc.egui_ctx);
            ui::ThemePalette::from_mode(false).apply(&cc.egui_ctx);
            Ok(Box::new(MyWordApp::default()))
        }),
    )
}
