//! Thumbnail UI regression tests for `viewkai-app`.

mod common;

use eframe::egui;
use egui_kittest::kittest::Queryable;
use std::sync::{Mutex, OnceLock};
use viewkai_core::PageIndex;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn ctrl_shift() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    }
}

#[test]
fn thumbnail_click_scrolls_to_page() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_500page();

    h.get_by_label("Single Page").click();
    h.run_ok();
    assert_eq!(
        h.state().viewer().visible_pages().first().copied(),
        Some(PageIndex(0))
    );

    h.key_press_modifiers(ctrl_shift(), egui::Key::T);
    h.run_ok();

    h.get_by_label("Page 2").click();
    h.run_ok();
    h.run_ok();

    assert_eq!(
        h.state().viewer().visible_pages().first().copied(),
        Some(PageIndex(1))
    );
}
