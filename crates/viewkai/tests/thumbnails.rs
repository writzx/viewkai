//! Viewer thumbnail integration tests.

use egui::Color32;
use egui_kittest::{Harness, kittest::Queryable};
use std::{
    cell::Cell,
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use viewkai::{Viewer, ViewerPlugin};
use viewkai_core::{PageIndex, PointsRect};
use viewkai_plugins::PluginContext;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn viewer_thumbnail_accessor_returns_plugin() {
    let viewer = Viewer::new();
    assert_eq!(viewer.thumbnails().id(), "viewkai.thumbnail");
}

#[test]
fn thumbnail_texture_cached_after_first_access() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load hello.pdf");

    struct State {
        viewer: Viewer,
        texture_id: Option<egui::TextureId>,
    }

    let mut harness = Harness::builder().build_ui_state(
        |ui, state: &mut State| {
            state.viewer.show(ui);
            state.texture_id = state
                .viewer
                .thumbnail_texture(ui, PageIndex(0))
                .map(|texture| texture.id());
        },
        State {
            viewer,
            texture_id: None,
        },
    );

    harness.run_steps(2);
    assert!(
        harness.state().texture_id.is_some(),
        "thumbnail should be cached"
    );

    harness.run_ok();
    assert!(harness.state().texture_id.is_some());
}

#[test]
fn thumbnail_click_scrolls_to_page() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let doc = viewkai_engine::Document::from_bytes(bytes).expect("load 500page.pdf");
    let mut plugin = viewkai::ThumbnailPlugin::new();
    let egui_ctx = egui::Context::default();
    let pending_scroll = Cell::new(None);

    plugin.set_visible(true);
    plugin.set_cache_budget(64 * 1024 * 1024);
    plugin.set_visible(true);
    plugin.set_visible(true);
    plugin.set_visible(true);

    let mut harness = Harness::builder().build_ui_state(
        |ui, plugin: &mut viewkai::ThumbnailPlugin| {
            plugin.render_panel(ui, Some(&doc));
        },
        plugin,
    );
    harness.run_ok();
    harness.get_by_label("Page 2").click();
    harness.run_ok();

    let rotations = HashMap::new();
    let mut ctx = PluginContext::new(
        Some(&doc),
        1.0,
        &[],
        &egui_ctx,
        Color32::WHITE,
        true,
        &rotations,
        None,
        &pending_scroll,
    );
    harness.state_mut().on_frame_update(&mut ctx);

    assert_eq!(
        pending_scroll.get(),
        Some((
            PageIndex(1),
            PointsRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        ))
    );
}

#[test]
fn thumbnails_500page_stress_stays_within_budget() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("Failed to initialize PDFium");

    let bytes = include_bytes!("../../../tests/fixtures/500page.pdf").to_vec();
    let mut viewer = Viewer::new();
    viewer.load_bytes(bytes).expect("load 500page.pdf");
    viewer.thumbnails_mut().set_cache_budget(64 * 1024 * 1024);

    let mut harness = Harness::builder().build_ui_state(
        |ui, viewer| {
            viewer.show(ui);
            for page in 0..viewer.page_count() {
                let _ = viewer.thumbnail_texture(ui, PageIndex(page));
            }
        },
        viewer,
    );

    harness.run_steps(170);
    assert!(harness.state().thumbnails().cache_bytes() <= 64 * 1024 * 1024);
}
