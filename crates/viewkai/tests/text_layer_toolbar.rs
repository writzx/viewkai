//! Tests for the text-layer plugin's toolbar contribution.

use egui_kittest::{Harness, kittest::Queryable};
use std::panic::{AssertUnwindSafe, catch_unwind};
use viewkai::Viewer;

fn make_harness_with_hello() -> Harness<'static, Viewer> {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");

    Harness::builder()
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
        )
}

#[test]
fn checkbox_toggles_plugin_state() {
    let mut harness = make_harness_with_hello();
    harness.run_ok();
    assert!(!harness.state().text_layer().debug());

    harness.get_by_label("Show text layer").click();
    harness.run_ok();
    assert!(harness.state().text_layer().debug());

    harness.get_by_label("Show text layer").click();
    harness.run_ok();
    assert!(!harness.state().text_layer().debug());
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
    harness.get_by_label("Show text layer");
}
