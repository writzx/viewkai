//! End-to-end selection tests through the Viewer API.

use std::sync::{Mutex, OnceLock};

use viewkai::Viewer;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn viewer_with_hello() -> Viewer {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");
    viewer
}

#[test]
fn viewer_select_all_and_copy() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut viewer = viewer_with_hello();
    viewer.select_all();
    let text = viewer.selected_text();
    assert!(
        !text.is_empty(),
        "selected text should be non-empty after select_all"
    );
}

#[test]
fn esc_deselects() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut viewer = viewer_with_hello();
    viewer.select_all();
    assert!(viewer.selection().is_some());
    viewer.deselect();
    assert!(viewer.selection().is_none());
}

#[test]
fn library_shortcuts_opt_out() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut viewer = viewer_with_hello();
    viewer.set_library_shortcuts_enabled(false);
    assert!(!viewer.library_shortcuts_enabled());
    assert!(!viewer.library_shortcuts_enabled());
}
