//! Application shell menu tests.

mod common;

use egui_kittest::kittest::Queryable;

#[test]
fn file_close_clears_viewer() {
    let mut h = common::demo_harness_with_hello();
    h.get_by_label("File").click();
    h.run_ok();
    h.get_by_label("Close (Ctrl+W)").click();
    h.run_ok();

    assert_eq!(h.state().viewer().page_count(), 0);
}

#[test]
fn file_open_url_opens_modal() {
    let mut h = common::demo_harness_with_hello();
    h.get_by_label("File").click();
    h.run_ok();
    h.get_by_label("Open from URL… (Ctrl+L)").click();
    h.run_ok();

    assert!(h.state().url_dialog_visible());
    h.get_by_label("Open from URL");
}

#[test]
fn view_menu_exposes_mode_radio() {
    let h = common::demo_harness_with_500page();
    h.get_by_label("Single Page");
}

#[test]
fn view_debug_toggles_text_layer() {
    let mut h = common::demo_harness_with_hello();
    assert!(!h.state().viewer().text_layer_debug());

    h.get_by_label("View").click();
    h.run_ok();
    h.get_by_label("Debug View").click();
    h.run_ok();

    h.get_by_label("Show text layer").click();
    h.run_ok();
    assert!(h.state().viewer().text_layer_debug());
}

#[test]
fn help_about_dialog_opens() {
    let mut h = common::demo_harness_with_hello();
    h.get_by_label("Help").click();
    h.run_ok();
    h.get_by_label("About viewkai").click();
    h.run_ok();

    assert!(h.state().about_visible());
    h.get_by_label("About viewkai");
}
