//! Debug-panel regression tests.

mod common;

use egui_kittest::kittest::Queryable;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn open_debug_view(h: &mut egui_kittest::Harness<'static, viewkai_app::App>) {
    h.get_by_label("View").click();
    h.run_ok();
    h.get_by_label("Debug View").click();
    h.run_ok();
}

#[test]
fn debug_panel_default_hidden() {
    let h = common::demo_harness_with_hello();

    assert!(!h.state().debug_panel_visible());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("Show text layer"))).is_err());
}

#[test]
fn debug_view_toggle() {
    let mut h = common::demo_harness_with_hello();
    assert!(!h.state().debug_panel_visible());

    open_debug_view(&mut h);
    assert!(h.state().debug_panel_visible());
    h.get_by_label("Show text layer");

    open_debug_view(&mut h);
    assert!(!h.state().debug_panel_visible());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("Show text layer"))).is_err());
}

#[test]
fn text_layer_in_debug_panel() {
    let mut h = common::demo_harness_with_hello();
    assert!(!h.state().viewer().text_layer_debug());

    open_debug_view(&mut h);
    h.get_by_label("Show text layer").click();
    h.run_ok();

    assert!(h.state().viewer().text_layer_debug());
}
