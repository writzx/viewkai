use eframe::egui;

#[derive(Default)]
struct DemoApp;

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("viewkai-demo");
            ui.label(format!("Loaded {}", viewkai::NAME));
        });
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
