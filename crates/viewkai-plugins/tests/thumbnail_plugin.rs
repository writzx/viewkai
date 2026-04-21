//! Thumbnail plugin tests.

use std::{cell::Cell, collections::HashMap, sync::OnceLock};

use egui::{Color32, Key, TextureHandle};
use egui_kittest::{Harness, kittest::Queryable};
use viewkai_core::{PageIndex, PointsRect};
use viewkai_engine::Document;
use viewkai_plugins::{PluginContext, ThumbnailPlugin, ViewerPlugin};

fn pdfium_once() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        viewkai_engine::init().expect("pdfium init");
    });
}

fn load_doc(path: &str) -> Document {
    let bytes = match path {
        "500page" => include_bytes!("../../../tests/fixtures/500page.pdf").to_vec(),
        _ => include_bytes!("../../../tests/fixtures/hello.pdf").to_vec(),
    };
    Document::from_bytes(bytes).expect("load test document")
}

struct CacheState {
    plugin: ThumbnailPlugin,
    doc: Document,
    pending_scroll: Cell<Option<(PageIndex, PointsRect)>>,
    requested_page: PageIndex,
    last_texture: Option<TextureHandle>,
}

impl CacheState {
    fn new(doc: Document, requested_page: PageIndex) -> Self {
        Self {
            plugin: ThumbnailPlugin::new(),
            doc,
            pending_scroll: Cell::new(None),
            requested_page,
            last_texture: None,
        }
    }
}

fn run_cache_frame(ui: &mut egui::Ui, state: &mut CacheState) {
    let rotations = HashMap::new();
    let mut ctx = PluginContext::new(
        Some(&state.doc),
        1.0,
        &[],
        ui.ctx(),
        Color32::WHITE,
        true,
        &rotations,
        None,
        &state.pending_scroll,
    );
    state.plugin.on_frame_update(&mut ctx);
    state.last_texture = state
        .plugin
        .thumbnail_texture(
            ui,
            &state.doc,
            state.requested_page,
            viewkai_core::PdfPageRotation::None,
        );
}

#[test]
fn thumbnail_cache_evicts_to_budget() {
    pdfium_once();

    let doc = load_doc("500page");
    let mut harness = Harness::builder().build_ui_state(run_cache_frame, CacheState::new(doc, PageIndex(0)));

    harness.state_mut().plugin.set_cache_budget(90_000);

    for page in [PageIndex(0), PageIndex(1), PageIndex(2), PageIndex(3)] {
        harness.state_mut().requested_page = page;
        harness.run_steps(2);
        assert!(harness.state().last_texture.is_some(), "page {} should render", page.0);
        assert!(harness.state().plugin.cache_bytes() <= 90_000);
    }
}

#[test]
fn thumbnail_cache_hit_updates_lru() {
    pdfium_once();

    let doc = load_doc("500page");
    let mut harness = Harness::builder().build_ui_state(run_cache_frame, CacheState::new(doc, PageIndex(0)));

    harness.state_mut().plugin.set_cache_budget(150_000);

    harness.state_mut().requested_page = PageIndex(0);
    harness.run_steps(2);
    let page0 = harness
        .state()
        .last_texture
        .as_ref()
        .map(TextureHandle::id)
        .expect("page 0 texture");

    harness.state_mut().requested_page = PageIndex(1);
    harness.run_steps(2);
    let page1 = harness
        .state()
        .last_texture
        .as_ref()
        .map(TextureHandle::id)
        .expect("page 1 texture");
    assert_ne!(page0, page1);

    harness.state_mut().requested_page = PageIndex(0);
    harness.run_steps(1);
    assert_eq!(
        harness.state().last_texture.as_ref().map(TextureHandle::id),
        Some(page0)
    );

    harness.state_mut().requested_page = PageIndex(2);
    harness.run_steps(2);

    harness.state_mut().requested_page = PageIndex(0);
    harness.run_steps(1);
    assert_eq!(
        harness.state().last_texture.as_ref().map(TextureHandle::id),
        Some(page0)
    );

    harness.state_mut().requested_page = PageIndex(1);
    harness.run_steps(1);
    assert!(harness.state().last_texture.is_none(), "page 1 should be evicted first");
}

#[test]
fn render_panel_click_queues_navigation() {
    pdfium_once();

    struct PanelState {
        plugin: ThumbnailPlugin,
        doc: Document,
    }

    let mut harness = Harness::builder().build_ui_state(
        |ui, state: &mut PanelState| {
            state.plugin.render_panel(ui, Some(&state.doc));
        },
        PanelState {
            plugin: ThumbnailPlugin::new(),
            doc: load_doc("500page"),
        },
    );
    harness.run_ok();

    harness.key_press(Key::Tab);
    harness.run_ok();
    harness.get_by_label("Page 2").click();
    harness.run_ok();

    assert_eq!(harness.state().plugin.pending_click_page(), Some(PageIndex(1)));
}
