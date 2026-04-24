//! Page navigation regression tests for `viewkai-app`.

mod common;

use eframe::egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use std::sync::{Mutex, OnceLock};
use viewkai_core::PageIndex;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn nav_button<'a>(h: &'a egui_kittest::Harness<'static, viewkai_app::App>, label: &'a str) -> egui_kittest::Node<'a> {
    h.query_by(|node| node.role() == Role::Button && node.label().as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing page nav button {label}"))
}

fn assert_visible_page(h: &egui_kittest::Harness<'static, viewkai_app::App>, expected_page_num: usize) {
    assert_eq!(
        h.state().viewer().visible_pages().first().copied(),
        Some(PageIndex(expected_page_num - 1))
    );
}

fn click_next_n(h: &mut egui_kittest::Harness<'static, viewkai_app::App>, times: usize) {
    for _ in 0..times {
        nav_button(h, ">").click();
        h.run_ok();
    }
}

fn set_single_page_mode(h: &mut egui_kittest::Harness<'static, viewkai_app::App>) {
    h.get_by_label("Single Page").click();
    h.run_ok();
}

#[test]
fn prev_button_disabled_on_first_page() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let h = common::demo_harness_with_hello();

    assert!(nav_button(&h, "<").accesskit_node().is_disabled());
}

#[test]
fn next_button_disabled_on_last_page() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let h = common::demo_harness_with_hello();

    assert!(nav_button(&h, ">").accesskit_node().is_disabled());
}

#[test]
fn prev_button_advances_backward() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_500page();

    set_single_page_mode(&mut h);
    click_next_n(&mut h, 4);
    assert_visible_page(&h, 5);

    nav_button(&h, "<").click();
    h.run_ok();

    assert_visible_page(&h, 4);
}

#[test]
fn next_button_advances_forward() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = common::demo_harness_with_500page();

    set_single_page_mode(&mut h);
    click_next_n(&mut h, 4);
    assert_visible_page(&h, 5);

    nav_button(&h, ">").click();
    h.run_ok();

    assert_visible_page(&h, 6);
}
