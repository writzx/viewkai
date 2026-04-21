//! Test-only helpers. NOT part of the supported public API. Gated behind
//! `--features test-support`. Used by `viewkai-plugins`' own integration
//! tests under `tests/` and by future plan tests that need to exercise
//! plugin hooks without standing up a full `Viewer`.

use std::cell::Cell;
use std::collections::HashMap;

use egui::{Color32, Context};
use viewkai_core::{PageIndex, PdfPageRotation, PointsRect};
use viewkai_engine::Document;

use crate::plugin::PluginContext;

/// Build a [`PluginContext`] for unit tests that only need to drive a
/// plugin's hooks. Callers own the backing `egui::Context`, `Cell`, and
/// optional `Document` reference.
pub fn build_context<'a>(
    document: Option<&'a Document>,
    zoom: f32,
    visible_pages: &'a [PageIndex],
    egui_ctx: &'a Context,
    selection_color: Color32,
    library_shortcuts_enabled: bool,
    rotations: &'a HashMap<PageIndex, PdfPageRotation>,
    pending_scroll: &'a Cell<Option<(PageIndex, PointsRect)>>,
) -> PluginContext<'a> {
    PluginContext {
        document,
        zoom,
        visible_pages,
        egui_ctx,
        selection_color,
        library_shortcuts_enabled,
        rotations,
        page_rect_screen: None,
        repaint_requested: false,
        pending_scroll,
    }
}
