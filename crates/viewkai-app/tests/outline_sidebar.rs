//! Outline sidebar tests.

mod common;

use eframe::egui;

fn ctrl_shift() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    }
}

#[test]
fn ctrl_shift_o_toggles_outline_panel() {
    let mut h = common::demo_harness_with_bookmarks();
    h.run_ok();
    assert!(!h.state().viewer().outline().visible());

    h.key_press_modifiers(ctrl_shift(), egui::Key::O);
    h.step();
    assert!(h.state().viewer().outline().visible());

    h.key_press_modifiers(ctrl_shift(), egui::Key::O);
    h.step();
    assert!(!h.state().viewer().outline().visible());
}
