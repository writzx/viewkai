//! Shortcut and load-state regression tests for `viewkai-app`.

mod common;

use eframe::egui;
use egui_kittest::kittest::Queryable;
use std::sync::{Mutex, OnceLock};
use viewkai::zoom::ZoomState;
use viewkai_app::LoadState;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        ..Default::default()
    }
}

#[test]
fn shortcut_ctrl_0_resets_zoom() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.key_press_modifiers(ctrl(), egui::Key::Equals);
    h.step();
    assert_eq!(h.state().viewer().zoom(), ZoomState::Discrete(1.25));

    h.key_press_modifiers(ctrl(), egui::Key::Num0);
    h.step();
    assert_eq!(h.state().viewer().zoom(), ZoomState::Discrete(1.0));
}

#[test]
fn shortcut_ctrl_1_sets_fitwidth() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.key_press_modifiers(ctrl(), egui::Key::Num1);
    h.step();

    assert_eq!(h.state().viewer().zoom(), ZoomState::FitWidth);
}

#[test]
fn shortcut_ctrl_2_sets_fitpage() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.key_press_modifiers(ctrl(), egui::Key::Num2);
    h.step();

    assert_eq!(h.state().viewer().zoom(), ZoomState::FitPage);
}

#[test]
fn shortcut_ctrl_plus_minus_steps_zoom() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.key_press_modifiers(ctrl(), egui::Key::Equals);
    h.step();
    assert_eq!(h.state().viewer().zoom(), ZoomState::Discrete(1.25));

    h.key_press_modifiers(ctrl(), egui::Key::Minus);
    h.step();
    assert_eq!(h.state().viewer().zoom(), ZoomState::Discrete(1.0));

    h.key_press_modifiers(ctrl(), egui::Key::Minus);
    h.step();
    assert_eq!(h.state().viewer().zoom(), ZoomState::Discrete(0.75));
}

#[test]
fn shortcut_ctrl_g_focuses_page_input_and_enter_preserves_loaded_doc() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_hello();

    h.key_press_modifiers(ctrl(), egui::Key::G);
    h.step();
    h.step();
    h.key_press_modifiers(egui::Modifiers::default(), egui::Key::Enter);
    h.step();

    assert!(matches!(h.state().load_state(), LoadState::Loaded));
    assert_eq!(h.state().viewer().page_count(), 1);
}

#[test]
fn demo_load_state_idle_to_loaded() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness();
    h.run_ok();

    assert!(matches!(h.state().load_state(), LoadState::Idle));

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    h.state_mut().load_bytes_sync(bytes).expect("load hello");
    h.run_ok();

    assert!(matches!(h.state().load_state(), LoadState::Loaded));
    assert_eq!(h.state().viewer().page_count(), 1);
}

#[test]
fn demo_load_state_failed_dismiss_returns_to_idle() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness();

    let err = h
        .state_mut()
        .load_bytes_sync(b"not a pdf".to_vec())
        .expect_err("invalid bytes should fail");
    assert!(!err.is_empty(), "failure should surface a message");
    h.run_ok();

    assert!(matches!(
        h.state().load_state(),
        LoadState::Failed { .. }
    ));
    h.get_by_label("Dismiss").click();
    h.run_ok();

    assert!(matches!(h.state().load_state(), LoadState::Idle));
    assert_eq!(h.state().viewer().page_count(), 0);
}

#[test]
fn demo_harness_helpers_load_expected_fixtures() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let hello = common::demo_harness_with_hello();
    assert!(matches!(hello.state().load_state(), LoadState::Loaded));
    assert_eq!(hello.state().viewer().page_count(), 1);

    let multi = common::demo_harness_with_500page();
    assert!(matches!(multi.state().load_state(), LoadState::Loaded));
    assert_eq!(multi.state().viewer().page_count(), 500);
}
