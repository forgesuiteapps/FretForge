#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod engine;
mod studio;
mod audio;
mod stems;
mod pitched;
mod ghl;
mod star_power;
mod timing;
mod lyrics;
mod fidelity;
fn app_icon()->eframe::egui::IconData {
    let image=image::load_from_memory(include_bytes!("../assets/fretforge.png")).expect("bundled icon").into_rgba8();
    eframe::egui::IconData {width:image.width(),height:image.height(),rgba:image.into_raw()}
}
fn main() -> eframe::Result<()> {
    if std::env::args().any(|a| a == "--smoke-test") {
        engine::smoke_test().expect("end-to-end conversion failed");
        return Ok(());
    }
    eframe::run_native("FretForge · Studio", eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_icon(std::sync::Arc::new(app_icon())).with_inner_size([1240.0, 980.0]).with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    }, Box::new(|cc| Ok(Box::new(studio::Studio::new(cc)))))
}
