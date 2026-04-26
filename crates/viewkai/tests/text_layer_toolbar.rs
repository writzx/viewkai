//! Tests for the text-layer plugin's toolbar contribution.
//!
//! Plan 03.25 Phase A: the "Show text layer" checkbox was moved from the plugin
//! toolbar into the app-shell debug panel. `show_plugin_toolbars` no longer
//! renders it; the toggle is now an app-shell concern (see viewkai-app/tests/debug_panel.rs).

use egui_kittest::{Harness, kittest::Queryable};
use std::panic::{AssertUnwindSafe, catch_unwind};
use viewkai::Viewer;

/// The text-layer debug state can be toggled via the public API directly.
#[test]
fn checkbox_toggles_plugin_state() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");

    assert!(!viewer.text_layer().debug());
    viewer.set_text_layer_debug(true);
    assert!(viewer.text_layer().debug());
    viewer.set_text_layer_debug(false);
    assert!(!viewer.text_layer().debug());
}

#[test]
fn show_does_not_invoke_toolbars() {
    viewkai_engine::init().expect("pdfium init");
    let viewer = Viewer::new();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(
            |ui, viewer| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    viewer.show(ui);
                });
            },
            viewer,
        );
    harness.run_ok();
    let result = catch_unwind(AssertUnwindSafe(|| harness.get_by_label("Show text layer")));
    assert!(result.is_err());
}

/// Plan 03.25 Phase A: the text-layer checkbox is no longer in the plugin toolbar.
/// `show_plugin_toolbars` must NOT render a "Show text layer" checkbox.
#[test]
fn show_plugin_toolbars_renders_checkbox() {
    viewkai_engine::init().expect("pdfium init");
    let viewer = Viewer::new();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(
            |ui, viewer| {
                egui::Panel::top("toolbar").show_inside(ui, |ui| {
                    viewer.show_plugin_toolbars(ui);
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    viewer.show(ui);
                });
            },
            viewer,
        );
    harness.run_ok();
    // The checkbox is now in the app-shell debug panel, not the plugin toolbar.
    let result = catch_unwind(AssertUnwindSafe(|| harness.get_by_label("Show text layer")));
    assert!(
        result.is_err(),
        "Show text layer must not appear in plugin toolbar"
    );
}
