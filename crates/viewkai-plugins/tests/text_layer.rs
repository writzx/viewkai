//! Text layer regression tests.

use std::{cell::Cell, collections::HashMap, sync::OnceLock};

use egui::{CentralPanel, Color32, RawInput, Rect, pos2, vec2};
use viewkai_core::{PageIndex, PdfPageRotation, PointsRect};
use viewkai_plugins::{PluginContext, TextLayerPlugin, ViewerPlugin};

fn pdfium_once() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        viewkai_engine::init().expect("pdfium init");
    });
}

#[test]
fn text_layer_web_bbox_sanity() {
    pdfium_once();

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = viewkai_engine::Document::from_bytes(bytes).expect("load hello.pdf");
    let page = PageIndex(0);
    let page_size = doc.page_size(page).expect("page size");
    let page_rect = Rect::from_min_size(
        pos2(32.0, 24.0),
        vec2(page_size.width_pt, page_size.height_pt),
    );
    let ctx = egui::Context::default();
    let pending_scroll = Cell::new(None);
    let rotations = HashMap::<PageIndex, PdfPageRotation>::new();
    let visible_pages = [page];
    let mut plugin = TextLayerPlugin::new();
    plugin.set_debug(true);

    let output = ctx.run(RawInput::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            let mut plugin_ctx = PluginContext::new(
                Some(&doc),
                1.0,
                &visible_pages,
                ctx,
                Color32::LIGHT_BLUE,
                true,
                &rotations,
                Some(page_rect),
                &pending_scroll,
            );
            plugin.draw_page_overlay(page, ui, &mut plugin_ctx);
        });
    });

    let mut saw_word_rect = false;
    for clipped in output.shapes {
        let egui::epaint::ClippedShape { shape, .. } = clipped;
        if let egui::Shape::Rect(rect_shape) = shape {
            if rect_shape.stroke.color == Color32::RED {
                saw_word_rect = true;
                assert!(rect_shape.rect.min.x >= page_rect.min.x);
                assert!(rect_shape.rect.min.y >= page_rect.min.y);
                assert!(rect_shape.rect.max.x <= page_rect.max.x);
                assert!(rect_shape.rect.max.y <= page_rect.max.y);
            }
        }
    }

    assert!(saw_word_rect, "expected debug word rectangles");
    assert_eq!(pending_scroll.get(), None::<(PageIndex, PointsRect)>);
}
