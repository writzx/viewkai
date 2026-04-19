//! Library-level search API tests.

use viewkai::Viewer;

#[test]
fn viewer_search_api_shim_delegates() {
    let mut viewer = Viewer::new();
    viewer.open_search();
    assert!(viewer.search().is_open());
    viewer.close_search();
    assert!(!viewer.search().is_open());
}

#[test]
fn viewer_current_match_returns_none_when_no_state() {
    let viewer = Viewer::new();
    assert!(viewer.current_match().is_none());
}
