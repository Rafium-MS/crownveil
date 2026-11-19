mod app;

use app::CrownveilApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Crownveil"),
        ..Default::default()
    };

    eframe::run_native(
        "Crownveil",
        options,
        Box::new(|_creation_context| Ok(Box::new(CrownveilApp::new()))),
    )
}
