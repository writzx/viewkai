//! Outline plugin tests.

use std::{cell::Cell, collections::HashMap};

use egui_kittest::{Harness, kittest::Queryable};
use viewkai_plugins::{OutlinePlugin, PluginContext, ViewerPlugin};

#[test]
#[allow(clippy::items_after_statements)]
fn outline_plugin_renders_tree() {
    viewkai_engine::init().expect("pdfium init");
    let bytes = include_bytes!("../../../tests/fixtures/bookmarks.pdf").to_vec();
    let doc = viewkai_engine::Document::from_bytes(bytes).expect("load bookmarks.pdf");

    struct State {
        plugin: OutlinePlugin,
        doc: viewkai_engine::Document,
    }

    let mut harness = Harness::builder().build_ui_state(
        |ui, state: &mut State| {
            state.plugin.render_panel(ui, Some(&state.doc));
        },
        State {
            plugin: OutlinePlugin::new(),
            doc,
        },
    );
    harness.run_ok();

    harness.get_by_label("Root 1");
    harness.get_by_label("Root 2");
    harness.get_by_label("Root 3");
}

#[test]
#[allow(clippy::items_after_statements)]
fn show_toolbar_toggles_outline_panel() {
    let mut plugin = OutlinePlugin::new();
    assert!(plugin.visible());
    plugin.set_visible(true);
    assert!(plugin.visible());
    plugin.set_visible(false);
    assert!(!plugin.visible());
}

#[test]
fn goto_destination_emits_expected_target() {
    let mut plugin = OutlinePlugin::new();
    let egui_ctx = egui::Context::default();
    let pending_scroll = Cell::new(None);

    plugin.set_pending_destination(viewkai_core::Destination {
        page: viewkai_core::PageIndex(2),
        position: Some(viewkai_core::DestPosition::Point {
            x_pt: 12.0,
            y_pt: 34.0,
        }),
    });

    let rotations = HashMap::new();
    let mut ctx = PluginContext::new(
        None,
        1.0,
        &[viewkai_core::PageIndex(1)],
        &egui_ctx,
        egui::Color32::WHITE,
        true,
        &rotations,
        None,
        &pending_scroll,
    );
    plugin.on_frame_update(&mut ctx);

    assert_eq!(plugin.pending_destination(), None);
    assert_eq!(
        pending_scroll.get(),
        Some((
            viewkai_core::PageIndex(2),
            viewkai_core::PointsRect {
                x: 12.0,
                y: 34.0,
                width: 1.0,
                height: 1.0,
            },
        ))
    );
}
