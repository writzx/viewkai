//! Web debug panel regression tests.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};
use viewkai_web::DemoApp;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn demo_harness() -> Harness<'static, DemoApp> {
    viewkai_engine::init().expect("pdfium init");
    Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 900.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(
            |ui, app| {
                let ctx = ui.ctx().clone();
                app.handle_shortcuts_for_testing(&ctx);
                app.poll_pending_load_for_testing();
                app.sync_viewport_title_for_testing(&ctx);

                egui::Panel::top("menu_bar").show_inside(ui, |ui| {
                    app.show_menu_bar_for_testing(ui);
                });
                egui::Panel::top("web_controls").show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        app.show_compact_controls_for_testing(ui);
                    });
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    app.viewer_for_testing().show(ui);
                });
                app.show_about_window_for_testing(&ctx);
                app.show_url_window_for_testing(&ctx);
            },
            DemoApp::new_for_testing(),
        )
}

#[test]
fn debug_panel_hidden_by_default() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let h = demo_harness();

    let result = catch_unwind(AssertUnwindSafe(|| h.get_by_label("Debug")));

    assert!(result.is_err(), "debug panel should be hidden by default");
}
