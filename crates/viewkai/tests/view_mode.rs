//! Integration tests covering `Viewer` view-mode state transitions.

use egui_kittest::Harness;
use viewkai::{ViewMode, Viewer};

fn loaded_harness() -> Harness<'static, Viewer> {
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

#[test]
fn viewer_switching_to_continuous_keeps_single_anchor_page() {
    let mut h = loaded_harness();
    h.state_mut().set_view_mode(ViewMode::Single);
    h.state_mut().scroll_to_page(17);
    h.run_ok();

    h.state_mut().set_view_mode(ViewMode::Continuous);
    h.run_ok();

    assert_eq!(
        h.state().visible_pages().first().copied(),
        Some(viewkai_core::PageIndex(17))
    );
}

#[test]
fn viewer_switching_to_continuous_keeps_spread_anchor_page() {
    let mut h = loaded_harness();
    h.state_mut().set_view_mode(ViewMode::Spread {
        cover_separate: true,
    });
    h.state_mut().scroll_to_page(22);
    h.run_ok();

    h.state_mut().set_view_mode(ViewMode::Continuous);
    h.run_ok();

    assert_eq!(
        h.state().visible_pages().first().copied(),
        Some(viewkai_core::PageIndex(21))
    );
}
