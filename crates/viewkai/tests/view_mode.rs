//! Integration tests covering `Viewer` view-mode state transitions.

use viewkai::{ViewMode, Viewer};

#[test]
fn viewer_default_view_mode_is_continuous() {
    let viewer = Viewer::new();
    assert_eq!(viewer.view_mode(), ViewMode::Continuous);
}

#[test]
fn viewer_set_view_mode_single() {
    let mut viewer = Viewer::new();
    viewer.set_view_mode(ViewMode::Single);
    assert_eq!(viewer.view_mode(), ViewMode::Single);
}

#[test]
fn viewer_set_view_mode_spread() {
    let mut viewer = Viewer::new();
    let mode = ViewMode::Spread {
        cover_separate: true,
    };
    viewer.set_view_mode(mode);
    assert_eq!(viewer.view_mode(), mode);
}

#[test]
fn viewer_set_view_mode_back_to_continuous() {
    let mut viewer = Viewer::new();
    viewer.set_view_mode(ViewMode::Single);
    viewer.set_view_mode(ViewMode::Spread {
        cover_separate: false,
    });
    viewer.set_view_mode(ViewMode::Continuous);
    assert_eq!(viewer.view_mode(), ViewMode::Continuous);
}
