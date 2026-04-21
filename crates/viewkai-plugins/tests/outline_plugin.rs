//! Outline plugin tests.

use std::cell::Cell;

use egui_kittest::{Harness, kittest::Queryable};
use viewkai_plugins::{OutlinePlugin, PluginContext, ViewerPlugin};

#[test]
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
fn show_toolbar_toggles_outline_panel() {
    let egui_ctx = egui::Context::default();
    let pending_scroll = Cell::new(None);

    struct State {
        plugin: OutlinePlugin,
        egui_ctx: egui::Context,
        pending_scroll: Cell<Option<(viewkai_core::PageIndex, viewkai_core::PointsRect)>>,
    }

    let mut harness = Harness::builder().build_ui_state(
        |ui, state: &mut State| {
            let mut ctx = PluginContext::new(
                None,
                1.0,
                &[],
                &state.egui_ctx,
                egui::Color32::WHITE,
                true,
                None,
                &state.pending_scroll,
            );
            state.plugin.show_toolbar(ui, &mut ctx);
        },
        State {
            plugin: OutlinePlugin::new(),
            egui_ctx,
            pending_scroll,
        },
    );
    harness.run_ok();
    assert!(!harness.state().plugin.visible());

    harness.get_by_label("Show Outline").click();
    harness.run_ok();
    assert!(harness.state().plugin.visible());
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

    let mut ctx = PluginContext::new(
        None,
        1.0,
        &[viewkai_core::PageIndex(1)],
        &egui_ctx,
        egui::Color32::WHITE,
        true,
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
