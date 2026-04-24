//! Debug panel regression tests for `viewkai-app`.

mod common;

use egui_kittest::kittest::Queryable;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn debug_panel_hidden_by_default() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let h = common::demo_harness();

    let result = catch_unwind(AssertUnwindSafe(|| h.get_by_label("Debug")));

    assert!(result.is_err(), "debug panel should be hidden by default");
}

#[test]
fn debug_view_toggle_shows_panel() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness();

    h.get_by_label("View").click();
    h.run_ok();
    h.get_by_label("Debug View").click();
    h.run_ok();

    h.get_by_label("Debug");
}

#[test]
fn no_sidebar_submenu() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness();

    h.get_by_label("View").click();
    h.run_ok();

    let result = catch_unwind(AssertUnwindSafe(|| h.get_by_label("Sidebar")));

    assert!(result.is_err(), "View menu should not expose a Sidebar submenu");
}
