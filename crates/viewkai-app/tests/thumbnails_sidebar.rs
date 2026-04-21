//! Thumbnails sidebar tests.

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
fn ctrl_shift_t_toggles_thumbnails() {
    let mut h = common::demo_harness_with_hello();
    h.run_ok();
    assert!(!h.state().viewer().thumbnails().visible());

    h.key_press_modifiers(ctrl_shift(), egui::Key::T);
    h.step();
    assert!(h.state().viewer().thumbnails().visible());

    h.key_press_modifiers(ctrl_shift(), egui::Key::T);
    h.step();
    assert!(!h.state().viewer().thumbnails().visible());
}
