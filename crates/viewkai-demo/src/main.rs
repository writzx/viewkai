use eframe::egui;

#[derive(Default)]
struct DemoApp;

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("viewkai-demo");
        ui.label(format!("Loaded {}", viewkai::NAME));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "viewkai-demo",
        options,
        Box::new(|_cc| Ok(Box::new(DemoApp))),
    )
}
