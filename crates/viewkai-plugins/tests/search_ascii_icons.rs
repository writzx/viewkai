//! Search overlay icon regression tests.

use std::{cell::Cell, collections::HashMap, sync::{Mutex, OnceLock}};

use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::{NodeT, Queryable}};
use viewkai_core::{PageIndex, PointsRect};
use viewkai_engine::Document;
use viewkai_plugins::{PluginContext, SearchPlugin, ViewerPlugin};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct State {
    plugin: SearchPlugin,
    doc: Document,
    pending_scroll: Cell<Option<(PageIndex, PointsRect)>>,
}

#[test]
fn find_overlay_uses_ascii_glyphs() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("pdfium init");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let mut harness = Harness::builder().build_ui_state(
        |ui, state: &mut State| {
            let egui_ctx = ui.ctx().clone();
            let rotations = HashMap::new();
            let mut ctx = PluginContext::new(
                Some(&state.doc),
                1.0,
                &[PageIndex(0)],
                &egui_ctx,
                egui::Color32::WHITE,
                true,
                &rotations,
                None,
                &state.pending_scroll,
            );
            state.plugin.on_frame_update(&mut ctx);
            state.plugin.show_toolbar(ui, &mut ctx);
            state.plugin.show_viewer_overlay(&egui_ctx, &mut ctx);
        },
        State {
            plugin: SearchPlugin::new(),
            doc,
            pending_scroll: Cell::new(None),
        },
    );
    harness.run_ok();

    harness.get_by_label("Find").click();
    harness.run_ok();

    let button_labels = harness
        .query_all_by_role(Role::Button)
        .filter_map(|node| node.accesskit_node().label())
        .collect::<Vec<_>>();

    assert!(
        button_labels.iter().all(|label| label.chars().all(|ch| ch < '\u{2500}')),
        "expected ASCII-only button labels, got {button_labels:?}"
    );
}
