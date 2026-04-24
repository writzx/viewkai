//! Page input synchronization regression tests for `viewkai-app`.

mod common;

use eframe::egui;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use std::sync::{Mutex, OnceLock};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn set_single_page_mode(h: &mut egui_kittest::Harness<'static, viewkai_app::App>) {
    h.get_by_label("Single Page").click();
    h.run_ok();
}

#[test]
fn page_input_auto_updates_on_scroll() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_500page();

    set_single_page_mode(&mut h);
    for _ in 0..9 {
        h.query_by(|node| node.role() == Role::Button && node.label().as_deref() == Some(">"))
            .expect("next page button")
            .click();
        h.run_ok();
    }

    assert_eq!(
        h.get_by_role(Role::TextInput).value().as_deref(),
        Some("10")
    );
}

#[test]
fn page_input_does_not_fight_user_edit() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.get_by_role(Role::TextInput).focus();
    h.run_ok();
    h.key_press(egui::Key::Backspace);
    h.run_ok();
    h.get_by_role(Role::TextInput).type_text("5");
    h.run_ok();

    assert_eq!(
        h.get_by_role(Role::TextInput).value().as_deref(),
        Some("5")
    );
}
