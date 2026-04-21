//! Integration tests covering single-page and spread viewing modes.

use egui::Key;
use egui_kittest::Harness;
use std::sync::{Mutex, OnceLock};
use viewkai::{ViewMode, Viewer};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn multipage_harness() -> Harness<'static, Viewer> {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load 500page.pdf");
    Harness::builder()
        .with_size(egui::Vec2::new(800.0, 600.0))
        .with_os(egui::os::OperatingSystem::Nix)
        .build_ui_state(|ui, viewer| viewer.show(ui), viewer)
}

#[test]
fn single_mode_pgdn_advances() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = multipage_harness();
    h.state_mut().set_view_mode(ViewMode::Single);
    h.run_ok();

    assert_eq!(h.state().visible_pages(), &[viewkai_core::PageIndex(0)]);

    h.key_press(Key::PageDown);
    h.step();

    assert_eq!(h.state().visible_pages(), &[viewkai_core::PageIndex(1)]);
}

#[test]
fn single_mode_wraps_at_last() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = multipage_harness();
    h.state_mut().set_view_mode(ViewMode::Single);
    h.state_mut().scroll_to_page(499);
    h.run_ok();

    assert_eq!(h.state().visible_pages(), &[viewkai_core::PageIndex(499)]);

    h.key_press(Key::PageDown);
    h.step();

    assert_eq!(h.state().visible_pages(), &[viewkai_core::PageIndex(499)]);
}

#[test]
fn spread_mode_cover_separate_true() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = multipage_harness();
    h.state_mut().set_view_mode(ViewMode::Spread {
        cover_separate: true,
    });
    h.run_ok();

    assert_eq!(h.state().visible_pages(), &[viewkai_core::PageIndex(0)]);

    h.state_mut().navigate_next_page();
    h.run_ok();

    assert_eq!(
        h.state().visible_pages(),
        &[viewkai_core::PageIndex(1), viewkai_core::PageIndex(2)]
    );
}

#[test]
fn spread_mode_cover_separate_false() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = multipage_harness();
    h.state_mut().set_view_mode(ViewMode::Spread {
        cover_separate: false,
    });
    h.run_ok();

    assert_eq!(
        h.state().visible_pages(),
        &[viewkai_core::PageIndex(0), viewkai_core::PageIndex(1)]
    );

    h.state_mut().navigate_next_page();
    h.run_ok();

    assert_eq!(
        h.state().visible_pages(),
        &[viewkai_core::PageIndex(2), viewkai_core::PageIndex(3)]
    );
}

#[test]
fn spread_mode_scroll_to_page_lands_on_spread() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut h = multipage_harness();
    h.state_mut().set_view_mode(ViewMode::Spread {
        cover_separate: true,
    });
    h.state_mut().scroll_to_page(2);
    h.run_ok();

    assert_eq!(
        h.state().visible_pages(),
        &[viewkai_core::PageIndex(1), viewkai_core::PageIndex(2)]
    );
}
