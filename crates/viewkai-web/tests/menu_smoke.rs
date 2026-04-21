//! Web menu smoke tests.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use viewkai_web::DemoApp;

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

fn demo_harness_with_hello() -> Harness<'static, DemoApp> {
    let mut h = demo_harness();
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    h.state_mut().load_bytes_sync(bytes).expect("load hello");
    h.run_ok();
    h
}

#[test]
fn web_menu_bar_has_three_menus() {
    let h = demo_harness_with_hello();
    h.get_by_label("File");
    h.get_by_label("View");
    h.get_by_label("Help");
}

#[test]
fn web_file_close_clears_viewer() {
    let mut h = demo_harness_with_hello();
    h.get_by_label("File").click();
    h.run_ok();
    h.get_by_label("Close (Ctrl+W)").click();
    h.run_ok();

    assert_eq!(h.state().viewer().page_count(), 0);
}

#[test]
fn web_view_debug_toggles_text_layer() {
    let mut h = demo_harness_with_hello();
    assert!(!h.state().viewer().text_layer_debug());

    h.get_by_label("Show text layer").click();
    h.run_ok();

    assert!(h.state().viewer().text_layer_debug());
}

#[test]
fn web_url_input_no_longer_in_top_bar() {
    let h = demo_harness_with_hello();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        h.get_by_label("https://example.com/document.pdf")
    }));

    assert!(result.is_err());
}
