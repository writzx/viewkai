//! Core plugin trait and context types.

use std::{any::Any, cell::Cell};

use egui::{Color32, Context, Ui};
use viewkai_core::{PageIndex, PointsPos, PointsRect};
use viewkai_engine::Document;

/// Context passed to every plugin hook each frame.
///
/// Provides read access to the current document, zoom level, visible pages,
/// and egui context. Plugins use this to read state and request repaints or
/// scroll operations.
pub struct PluginContext<'a> {
    /// The currently loaded document, if any.
    pub document: Option<&'a Document>,
    /// Current effective zoom factor (1.0 = 100%).
    pub zoom: f32,
    /// Pages currently visible in the scroll area.
    pub visible_pages: &'a [PageIndex],
    /// The egui context for this frame.
    pub egui_ctx: &'a Context,
    /// Selection highlight color (from `Viewer::set_selection_color`).
    pub selection_color: Color32,
    /// Whether the library's built-in keyboard shortcuts are enabled.
    pub library_shortcuts_enabled: bool,
    /// Set by `request_repaint`; `Viewer` reads it at end of frame and calls
    /// `ctx.request_repaint()` if true.
    pub(crate) repaint_requested: bool,
    /// Shared reference into the `Viewer`-owned pending-scroll slot. `Cell`
    /// (single-threaded interior mutability) is chosen because `viewkai` is
    /// single-threaded on WASM and native (matching the rest of the library).
    pub(crate) pending_scroll: &'a Cell<Option<(PageIndex, PointsRect)>>,
}

impl PluginContext<'_> {
    /// Construct a new plugin context for one dispatch pass.
    #[must_use]
    pub fn new<'a>(
        document: Option<&'a Document>,
        zoom: f32,
        visible_pages: &'a [PageIndex],
        egui_ctx: &'a Context,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        pending_scroll: &'a Cell<Option<(PageIndex, PointsRect)>>,
    ) -> PluginContext<'a> {
        PluginContext {
            document,
            zoom,
            visible_pages,
            egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            repaint_requested: false,
            pending_scroll,
        }
    }

    /// Request that the viewer repaints on the next frame.
    pub fn request_repaint(&mut self) {
        self.repaint_requested = true;
    }

    /// Return whether a repaint was requested during this dispatch pass.
    #[must_use]
    pub fn repaint_requested(&self) -> bool {
        self.repaint_requested
    }

    /// Queue a "scroll viewer to show this rect on this page" request.
    ///
    /// `rect_in_page_pt` is in page-local points (same coordinate space as
    /// `GlyphBox::bbox`). The request writes to the `Viewer`-owned
    /// `pending_scroll` slot, which `Viewer::show_pages` drains on the NEXT
    /// frame's page pass. Later calls in the same frame overwrite earlier ones;
    /// the plugin rendering order determines priority.
    pub fn request_scroll_to(&self, page: PageIndex, rect_in_page_pt: PointsRect) {
        self.pending_scroll.set(Some((page, rect_in_page_pt)));
    }
}

/// A pointer (mouse/touch) event delivered to plugins in page-local coordinates.
pub struct PointerEvent {
    /// Pointer position in page-local PDF points.
    pub pos_in_page_pt: PointsPos,
    /// Whether the primary mouse button is currently held down.
    pub primary_down: bool,
    /// Active keyboard modifiers.
    pub modifiers: egui::Modifiers,
    /// Click count: 1 = single, 2 = double, 3 = triple.
    pub click_count: u8,
}

/// The sealed plugin trait for viewkai built-in plugins.
///
/// All hooks have empty default implementations so a plugin can override only
/// the surfaces it needs. The trait is sealed via the private `Sealed`
/// supertrait — only types inside `viewkai-plugins` can implement it.
///
/// ```compile_fail
/// use viewkai_plugins::{PluginContext, ViewerPlugin};
///
/// struct ExternalPlugin;
///
/// impl ViewerPlugin for ExternalPlugin {
///     fn id(&self) -> &'static str {
///         "external.plugin"
///     }
/// }
/// ```
pub trait ViewerPlugin: crate::sealed::Sealed + Any + 'static {
    /// A unique, stable identifier for this plugin (e.g. `"viewkai.text_layer"`).
    fn id(&self) -> &'static str;

    /// Called once when the plugin is registered with a `Viewer`.
    fn on_register(&mut self, _ctx: &mut PluginContext<'_>) {}

    /// Called once per frame before any page rendering.
    fn on_frame_update(&mut self, _ctx: &mut PluginContext<'_>) {}

    /// Called for each visible page after the page texture is drawn.
    /// `ui` is positioned at the page's top-left in screen space.
    fn draw_page_overlay(
        &mut self,
        _page: PageIndex,
        _ui: &mut Ui,
        _ctx: &mut PluginContext<'_>,
    ) {
    }

    /// Called to render toolbar contributions. Consumer decides placement by
    /// calling `Viewer::show_plugin_toolbars(ui)`.
    fn show_toolbar(&mut self, _ui: &mut Ui, _ctx: &mut PluginContext<'_>) {}

    /// Called to render viewer-level overlays (e.g. floating search bar).
    /// Consumer surfaces this by calling `Viewer::show_plugin_overlays(ctx)`.
    fn show_viewer_overlay(&mut self, _egui_ctx: &Context, _ctx: &mut PluginContext<'_>) {}

    /// Called for each pointer event on a page. Returns `true` if the event
    /// was consumed and should not be forwarded to other plugins.
    fn on_pointer_event(
        &mut self,
        _page: PageIndex,
        _event: &PointerEvent,
        _ctx: &mut PluginContext<'_>,
    ) -> bool {
        false
    }
}
