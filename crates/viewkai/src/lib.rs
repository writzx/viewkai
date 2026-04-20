//! viewkai — embeddable PDF viewer widget for egui.
//!
//! # Usage
//!
//! ```no_run
//! # use viewkai::Viewer;
//! let mut viewer = Viewer::new();
//! # let pdf_bytes = Vec::new();
//! viewer.load_bytes(pdf_bytes).expect("failed to open PDF");
//! // In your egui frame:
//! // viewer.show(ui);
//! ```

#![warn(missing_docs)]

pub mod cache;
pub mod error;
pub mod viewport;
pub mod zoom;

/// Canonical crate name exposed for embedding integrations.
pub const NAME: &str = "viewkai";

use crate::cache::{CacheKey, TextureCache};
use crate::error::LoadError;
use crate::viewport::{VisibilityTracker, VisibleSet};
use crate::zoom::ZoomState;
use egui::{Color32, Rect, Sense, TextureOptions, Vec2};
use std::{cell::Cell, sync::Arc};
use viewkai_core::{PageIndex, PageText, PointsRect, SelectionRange};
use viewkai_engine::{Document, error::EngineError};
use viewkai_plugins::PluginRegistry;

pub use viewkai_plugins::{
    PluginContext, PointerEvent, SearchPlugin, TextLayerPlugin, ViewerPlugin,
};

/// Per-page rendering state.
#[derive(Clone, Copy)]
pub struct PageState {
    /// Page dimensions in PDF points (1 pt = 1/72 inch).
    pub size_pt: Vec2,
}

/// Display state for the viewer.
enum ViewerState {
    Empty,
    Loaded {
        document: Arc<Document>,
        pages: Vec<PageState>,
    },
    Error(LoadError),
}

/// Internal rendering state: texture cache, visibility tracking, and zoom.
///
/// Extracted from [`Viewer`] to narrow responsibilities per Rust API Guidelines
/// C-STRUCT-PRIVATE.
struct RenderState {
    cache: TextureCache,
    visibility: VisibilityTracker,
    zoom: ZoomState,
}

impl RenderState {
    fn new() -> Self {
        Self {
            cache: TextureCache::default_budget(),
            visibility: VisibilityTracker::new(2),
            zoom: ZoomState::default(),
        }
    }
}

/// An embeddable PDF viewer widget.
///
/// Maintains its own document and texture state. Call [`Self::show`] each frame
/// to render the widget into the provided [`egui::Ui`].
pub struct Viewer {
    state: ViewerState,
    render: RenderState,
    pending_scroll_to_page: Option<usize>,
    plugins: PluginRegistry,
    pending_scroll: Cell<Option<(PageIndex, PointsRect)>>,
    selection_color: Color32,
    library_shortcuts_enabled: bool,
    last_visible_pages: Vec<PageIndex>,
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Viewer {
    /// Create a new, empty viewer.
    #[must_use]
    pub fn new() -> Self {
        let mut viewer = Self {
            state: ViewerState::Empty,
            render: RenderState::new(),
            pending_scroll_to_page: None,
            plugins: PluginRegistry::new(vec![
                Box::new(TextLayerPlugin::new()),
                Box::new(SearchPlugin::new()),
            ]),
            pending_scroll: Cell::new(None),
            selection_color: Color32::from_rgba_unmultiplied(70, 120, 210, 96),
            library_shortcuts_enabled: true,
            last_visible_pages: Vec::new(),
        };
        viewer.register_plugins();
        viewer
    }

    /// Set the active zoom mode for subsequent renders.
    pub fn set_zoom(&mut self, zoom: ZoomState) {
        self.render.zoom = zoom;
    }

    /// Return the currently configured zoom mode.
    #[must_use]
    pub fn zoom(&self) -> ZoomState {
        self.render.zoom
    }

    /// Scroll the viewer to make page `idx` visible.
    ///
    /// This sets a pending scroll target that is applied on the next `show()` call.
    pub fn scroll_to_page(&mut self, idx: usize) {
        self.pending_scroll_to_page = Some(idx);
    }

    /// Returns a shared reference to the built-in text-layer plugin.
    ///
    /// # Panics
    ///
    /// Panics if the text-layer plugin is not registered.
    #[must_use]
    pub fn text_layer(&self) -> &TextLayerPlugin {
        match self.plugins.get::<TextLayerPlugin>() {
            Some(plugin) => plugin,
            None => panic!("TextLayerPlugin is always registered"),
        }
    }

    /// Returns a mutable reference to the built-in text-layer plugin.
    ///
    /// # Panics
    ///
    /// Panics if the text-layer plugin is not registered.
    pub fn text_layer_mut(&mut self) -> &mut TextLayerPlugin {
        match self.plugins.get_mut::<TextLayerPlugin>() {
            Some(plugin) => plugin,
            None => panic!("TextLayerPlugin is always registered"),
        }
    }

    /// Returns a shared reference to the built-in search plugin.
    ///
    /// # Panics
    ///
    /// Panics if the search plugin is not registered.
    #[must_use]
    pub fn search(&self) -> &SearchPlugin {
        match self.plugins.get::<SearchPlugin>() {
            Some(plugin) => plugin,
            None => panic!("SearchPlugin is always registered"),
        }
    }

    /// Returns a mutable reference to the built-in search plugin.
    ///
    /// # Panics
    ///
    /// Panics if the search plugin is not registered.
    pub fn search_mut(&mut self) -> &mut SearchPlugin {
        match self.plugins.get_mut::<SearchPlugin>() {
            Some(plugin) => plugin,
            None => panic!("SearchPlugin is always registered"),
        }
    }

    /// Open the search overlay.
    pub fn open_search(&mut self) {
        self.search_mut().open();
    }

    /// Close the search overlay.
    pub fn close_search(&mut self) {
        self.search_mut().close();
    }

    /// Return the current search state, if any.
    #[must_use]
    pub fn search_state(&self) -> Option<&viewkai_core::SearchState> {
        self.search().state()
    }

    /// Advance to the next search match and return it.
    pub fn next_match(&mut self) -> Option<viewkai_core::SearchMatch> {
        self.search_mut().next_match().cloned()
    }

    /// Go to the previous search match and return it.
    pub fn prev_match(&mut self) -> Option<viewkai_core::SearchMatch> {
        self.search_mut().prev_match().cloned()
    }

    /// Return the current search match.
    #[must_use]
    pub fn current_match(&self) -> Option<viewkai_core::SearchMatch> {
        self.search().current_match().cloned()
    }

    /// Set the color for non-current match highlights.
    pub fn set_match_color(&mut self, color: egui::Color32) {
        self.search_mut().set_match_color(color);
    }

    /// Set the color for the current match highlight.
    pub fn set_current_match_color(&mut self, color: egui::Color32) {
        self.search_mut().set_current_match_color(color);
    }

    /// Set the color used to highlight selected text.
    ///
    /// Defaults to a semi-transparent blue matching egui's selection color.
    pub fn set_selection_color(&mut self, color: Color32) {
        self.selection_color = color;
    }

    /// Return the current selection highlight color.
    #[must_use]
    pub fn selection_color(&self) -> Color32 {
        self.selection_color
    }

    /// Enable or disable the library's built-in keyboard shortcuts.
    ///
    /// When disabled, plugins skip shortcut consumption entirely. Consumers
    /// can still call plugin methods directly. Defaults to `true`.
    pub fn set_library_shortcuts_enabled(&mut self, enabled: bool) {
        self.library_shortcuts_enabled = enabled;
    }

    /// Return whether the library's built-in keyboard shortcuts are enabled.
    #[must_use]
    pub fn library_shortcuts_enabled(&self) -> bool {
        self.library_shortcuts_enabled
    }

    /// Select all text in the loaded document.
    pub fn select_all(&mut self) {
        let egui_ctx = egui::Context::default();
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.clone();
        let pending_scroll = Cell::new(None);
        let ctx = Self::make_plugin_context(
            document,
            zoom,
            &visible_pages,
            &egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            None,
            &pending_scroll,
        );
        self.text_layer_mut().select_all(&ctx);
    }

    /// Clear the current text selection.
    pub fn deselect(&mut self) {
        self.text_layer_mut().deselect();
    }

    /// Return the currently selected text, if any.
    #[must_use]
    pub fn selected_text(&self) -> String {
        let egui_ctx = egui::Context::default();
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let ctx = Self::make_plugin_context(
            document,
            zoom,
            &[],
            &egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            None,
            &self.pending_scroll,
        );
        self.text_layer().selected_text(&ctx)
    }

    /// Copy the selected text to the clipboard.
    pub fn copy_selected_text(&self, egui_ctx: &egui::Context) {
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let ctx = Self::make_plugin_context(
            document,
            zoom,
            &[],
            egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            None,
            &self.pending_scroll,
        );
        self.text_layer().copy_selected_text(&ctx);
    }

    /// Return the current selection range, if any.
    #[must_use]
    pub fn selection(&self) -> Option<SelectionRange> {
        self.text_layer().selection().cloned()
    }

    /// Render toolbar contributions from all registered plugins.
    ///
    /// Call this inside a panel or toolbar area of your choice. [`Viewer::show`]
    /// does **not** call this — toolbar placement is a consumer UX decision.
    pub fn show_plugin_toolbars(&mut self, ui: &mut egui::Ui) {
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.as_slice();
        let selection_color = self.selection_color;
        let library_shortcuts_enabled = self.library_shortcuts_enabled;
        let pending_scroll = &self.pending_scroll;
        let egui_ctx = ui.ctx().clone();
        let mut ctx = Self::make_plugin_context(
            document,
            zoom,
            visible_pages,
            &egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            None,
            pending_scroll,
        );

        for plugin in &mut self.plugins {
            plugin.show_toolbar(ui, &mut ctx);
        }

        if ctx.repaint_requested() {
            egui_ctx.request_repaint();
        }
    }

    /// Render viewer-level overlays from all registered plugins.
    ///
    /// Call this once per frame, typically after page rendering. [`Viewer::show`]
    /// calls this automatically.
    pub fn show_plugin_overlays(&mut self, egui_ctx: &egui::Context) {
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.as_slice();
        let selection_color = self.selection_color;
        let library_shortcuts_enabled = self.library_shortcuts_enabled;
        let pending_scroll = &self.pending_scroll;
        let mut ctx = Self::make_plugin_context(
            document,
            zoom,
            visible_pages,
            egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            None,
            pending_scroll,
        );

        for plugin in &mut self.plugins {
            plugin.show_viewer_overlay(egui_ctx, &mut ctx);
        }

        if ctx.repaint_requested() {
            egui_ctx.request_repaint();
        }
    }

    /// Load a PDF from raw bytes.
    ///
    /// Parses the document synchronously. On Plan 01 fixtures (< 50 pages,
    /// < 5 MB) this is sub-100 ms on WASM.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes cannot be parsed as a PDF, or if the
    /// `PDFium` engine has not been initialised (call [`viewkai_engine::init()`]
    /// first).
    pub fn load_bytes(&mut self, bytes: Vec<u8>) -> Result<(), LoadError> {
        match Document::from_bytes(bytes) {
            Ok(doc) => {
                let count = doc.page_count();
                let pages = (0..count)
                    .map(|i| {
                        let size = doc
                            .page_size(PageIndex(i))
                            .map_or(Vec2::new(612.0, 792.0), |page| {
                                Vec2::new(page.width_pt, page.height_pt)
                            });

                        PageState { size_pt: size }
                    })
                    .collect();

                self.render.cache.clear();
                self.pending_scroll_to_page = None;
                self.pending_scroll.set(None);
                self.last_visible_pages.clear();
                self.state = ViewerState::Loaded {
                    document: Arc::new(doc),
                    pages,
                };

                Ok(())
            }
            Err(err) => {
                let load_err = match err {
                    EngineError::InvalidPdf => LoadError::InvalidPdf {
                        source: EngineError::InvalidPdf.to_string(),
                    },
                    EngineError::NotInitialised => LoadError::Uninitialised,
                    other => LoadError::Engine(other),
                };
                self.state = ViewerState::Error(load_err.clone());
                Err(load_err)
            }
        }
    }

    /// Drop the document and all textures, returning to the empty state.
    pub fn clear(&mut self) {
        self.state = ViewerState::Empty;
        self.render.cache.clear();
        self.pending_scroll_to_page = None;
        self.pending_scroll.set(None);
        self.last_visible_pages.clear();
    }

    /// Returns the number of pages in the loaded document, or 0 if no document is loaded.
    #[must_use]
    pub fn page_count(&self) -> usize {
        match &self.state {
            ViewerState::Loaded { pages, .. } => pages.len(),
            _ => 0,
        }
    }

    /// Returns the total bytes currently held in the texture cache.
    #[must_use]
    pub fn cache_bytes(&self) -> usize {
        self.render.cache.total_bytes()
    }

    /// Returns the page size in PDF points for the given index, if loaded.
    #[must_use]
    pub fn page_size_pt(&self, idx: usize) -> Option<Vec2> {
        match &self.state {
            ViewerState::Loaded { pages, .. } => pages.get(idx).map(|page| page.size_pt),
            _ => None,
        }
    }

    /// Return the extracted text for the given page index, if a document is loaded.
    ///
    /// Uses the document's text cache — extraction happens on first access per page.
    /// Returns `None` if no document is loaded.
    #[must_use]
    pub fn page_text(&self, idx: PageIndex) -> Option<Arc<PageText>> {
        match &self.state {
            ViewerState::Loaded { document, .. } => document.page_text(idx).ok(),
            ViewerState::Empty | ViewerState::Error(_) => None,
        }
    }

    /// Enable or disable the text-layer debug overlay (word bounding boxes).
    pub fn set_text_layer_debug(&mut self, enabled: bool) {
        self.text_layer_mut().set_debug(enabled);
    }

    /// Return whether the text-layer debug overlay is enabled.
    #[must_use]
    pub fn text_layer_debug(&self) -> bool {
        self.text_layer().debug()
    }

    /// Render the viewer into the given [`egui::Ui`].
    ///
    /// - **Empty**: shows "No document loaded".
    /// - **Error**: shows the error message and a "Retry" button.
    /// - **Loaded**: shows all pages in a vertical scroll area with lazy
    ///   rasterization.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.dispatch_frame_update(ui.ctx());

        let mut should_clear = false;

        match &mut self.state {
            ViewerState::Empty => {
                ui.centered_and_justified(|ui| {
                    ui.label("No document loaded");
                });
            }
            ViewerState::Error(err) => {
                let message = err.to_string();

                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(message).color(Color32::RED));

                        if ui.button("Retry").clicked() {
                            should_clear = true;
                        }
                    });
                });
            }
            ViewerState::Loaded { document, pages } => {
                Self::show_pages(
                    ui,
                    document,
                    pages,
                    &mut self.render.cache,
                    &self.render.visibility,
                    self.render.zoom,
                    &mut self.pending_scroll_to_page,
                    &mut self.plugins,
                    &mut self.last_visible_pages,
                    self.selection_color,
                    self.library_shortcuts_enabled,
                    &self.pending_scroll,
                );
            }
        }

        if should_clear {
            self.clear();
        }

        self.show_plugin_overlays(ui.ctx());
    }

    // justify: page rendering stays snapshot-stable when the existing render inputs
    // and plugin-dispatch state are passed explicitly instead of introducing a new struct.
    #[allow(clippy::too_many_arguments)]
    fn show_pages(
        ui: &mut egui::Ui,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        visibility: &VisibilityTracker,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
        plugins: &mut PluginRegistry,
        last_visible_pages: &mut Vec<PageIndex>,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        pending_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let available_width = ui.available_width();
                let viewport_rect = ui.clip_rect();
                let now = ui.input(|i| i.time);

                let effective_zoom = if let Some(first) = pages.first() {
                    zoom.effective_zoom(
                        available_width,
                        viewport_rect.height(),
                        first.size_pt.x,
                        first.size_pt.y,
                    )
                } else {
                    1.0
                };

                let dpi = ZoomState::zoom_to_dpi_bucket(effective_zoom);
                let zoom_bucket = ZoomState::dpi_to_bucket_index(dpi);

                let (page_tops, page_heights) = Self::compute_page_layout(pages, effective_zoom);
                Self::handle_pending_scroll(
                    ui,
                    viewer_pending_scroll,
                    pending_scroll,
                    &page_tops,
                    pages,
                    effective_zoom,
                    available_width,
                );

                let scroll_offset = ui.clip_rect().min.y - ui.min_rect().min.y;
                let vis_set = visibility.compute(
                    scroll_offset.max(0.0),
                    viewport_rect.height(),
                    &page_tops,
                    &page_heights,
                );
                last_visible_pages.clear();
                last_visible_pages.extend(vis_set.pages.iter().copied());

                let center_y = scroll_offset + viewport_rect.height() / 2.0;
                let to_render = Self::prioritize_renders(&vis_set, &page_tops, center_y);
                Self::render_queued_pages(ui, document, cache, &to_render, zoom_bucket, dpi, now);

                let egui_ctx = ui.ctx().clone();
                let mut ctx = Self::make_plugin_context(
                    Some(document.as_ref()),
                    effective_zoom,
                    last_visible_pages.as_slice(),
                    &egui_ctx,
                    selection_color,
                    library_shortcuts_enabled,
                    None,
                    pending_scroll,
                );

                Self::paint_pages(
                    ui,
                    pages,
                    cache,
                    &vis_set,
                    effective_zoom,
                    zoom_bucket,
                    available_width,
                    now,
                    plugins,
                    &mut ctx,
                );

                if ctx.repaint_requested() {
                    egui_ctx.request_repaint();
                }
            });
    }

    fn compute_page_layout(pages: &[PageState], effective_zoom: f32) -> (Vec<f32>, Vec<f32>) {
        const GAP: f32 = 16.0;

        let mut page_tops = Vec::with_capacity(pages.len());
        let mut cumulative_y = 0.0_f32;
        for page in pages {
            page_tops.push(cumulative_y);
            cumulative_y += page.size_pt.y * effective_zoom + GAP;
        }

        let page_heights = pages
            .iter()
            .map(|page| page.size_pt.y * effective_zoom)
            .collect();

        (page_tops, page_heights)
    }

    fn prioritize_renders(vis_set: &VisibleSet, page_tops: &[f32], center_y: f32) -> Vec<usize> {
        let center_page = page_tops
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (*a - center_y).abs();
                let db = (*b - center_y).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(0, |(i, _)| i);

        let mut to_render: Vec<usize> = vis_set.all_to_render().map(|page| page.0).collect();
        to_render.sort_by_key(|&idx| idx.abs_diff(center_page));
        to_render
    }

    fn render_queued_pages(
        ui: &egui::Ui,
        document: &Arc<Document>,
        cache: &mut TextureCache,
        to_render: &[usize],
        zoom_bucket: u8,
        dpi: u32,
        now: f64,
    ) {
        for &idx in to_render {
            let key = CacheKey {
                page_idx: PageIndex(idx),
                zoom_bucket,
            };

            if cache.get(&key, now).is_none()
                && let Ok(raw) = viewkai_engine::render_page(document, PageIndex(idx), dpi)
            {
                let byte_size = raw.pixels.len();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raw.width as usize, raw.height as usize],
                    &raw.pixels,
                );
                let handle = ui.ctx().load_texture(
                    format!("viewkai/page/{idx}/dpi{dpi}"),
                    image,
                    TextureOptions::LINEAR,
                );
                cache.insert(key, handle, byte_size, now);
            }
        }
    }

    // justify: this helper keeps page painting snapshot-stable by accepting the
    // already-computed render inputs directly instead of introducing a new struct.
    #[allow(clippy::too_many_arguments)]
    fn paint_pages(
        ui: &mut egui::Ui,
        pages: &[PageState],
        cache: &mut TextureCache,
        vis_set: &VisibleSet,
        effective_zoom: f32,
        zoom_bucket: u8,
        available_width: f32,
        now: f64,
        plugins: &mut PluginRegistry,
        plugin_ctx: &mut PluginContext<'_>,
    ) {
        const GAP: f32 = 16.0;
        const PLACEHOLDER_FILL: Color32 = Color32::from_gray(220);

        for (idx, page) in pages.iter().enumerate() {
            let display_size = Vec2::new(
                page.size_pt.x * effective_zoom,
                page.size_pt.y * effective_zoom,
            );
            let x_offset = ((available_width - display_size.x) / 2.0).max(0.0);
            let (row_rect, response) = ui.allocate_exact_size(
                Vec2::new(available_width, display_size.y + GAP),
                Sense::click_and_drag(),
            );
            let page_rect =
                Rect::from_min_size(row_rect.min + Vec2::new(x_offset, 0.0), display_size);

            let key = CacheKey {
                page_idx: PageIndex(idx),
                zoom_bucket,
            };

            if let Some(texture) = cache.get(&key, now) {
                ui.painter().image(
                    texture.id(),
                    page_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                let fallback = (0..6_u8).find_map(|bucket| {
                    let fallback_key = CacheKey {
                        page_idx: PageIndex(idx),
                        zoom_bucket: bucket,
                    };
                    cache.get(&fallback_key, now).map(egui::TextureHandle::id)
                });

                if let Some(tex_id) = fallback {
                    ui.painter().image(
                        tex_id,
                        page_rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().rect_filled(page_rect, 0.0, PLACEHOLDER_FILL);
                    if vis_set.pages.contains(&PageIndex(idx)) {
                        egui::Spinner::new().paint_at(ui, page_rect);
                    }
                }
            }

            let page_index = PageIndex(idx);
            if vis_set.pages.contains(&page_index) {
                if let Some(pointer_event) =
                    Self::pointer_event(ui, &response, page_rect, effective_zoom)
                {
                    for plugin in &mut *plugins {
                        if plugin.on_pointer_event(page_index, &pointer_event, plugin_ctx) {
                            break;
                        }
                    }
                }

                plugin_ctx.page_rect_screen = Some(page_rect);
                let mut overlay_ui = ui.new_child(egui::UiBuilder::new().max_rect(page_rect));
                for plugin in &mut *plugins {
                    plugin.draw_page_overlay(page_index, &mut overlay_ui, plugin_ctx);
                }
                plugin_ctx.page_rect_screen = None;
            }
        }
    }

    fn handle_pending_scroll(
        ui: &mut egui::Ui,
        pending_scroll_to_page: &mut Option<usize>,
        pending_plugin_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
        page_tops: &[f32],
        pages: &[PageState],
        effective_zoom: f32,
        available_width: f32,
    ) {
        if let Some(target_idx) = pending_scroll_to_page.take()
            && let Some(&top) = page_tops.get(target_idx)
        {
            ui.scroll_to_rect(
                Rect::from_min_size(egui::pos2(0.0, top), Vec2::new(1.0, 1.0)),
                Some(egui::Align::TOP),
            );
        } else if let Some((page, rect_in_page_pt)) = pending_plugin_scroll.take()
            && let Some((&top, page_state)) = page_tops.get(page.0).zip(pages.get(page.0))
        {
            let page_width = page_state.size_pt.x * effective_zoom;
            let x_offset = ((available_width - page_width) / 2.0).max(0.0);
            let target_rect = Rect::from_min_size(
                egui::pos2(
                    x_offset + rect_in_page_pt.x * effective_zoom,
                    top + rect_in_page_pt.y * effective_zoom,
                ),
                Vec2::new(
                    rect_in_page_pt.width.max(1.0) * effective_zoom,
                    rect_in_page_pt.height.max(1.0) * effective_zoom,
                ),
            );
            ui.scroll_to_rect(target_rect, Some(egui::Align::Center));
        }
    }

    fn register_plugins(&mut self) {
        let egui_ctx = egui::Context::default();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.as_slice();
        let selection_color = self.selection_color;
        let library_shortcuts_enabled = self.library_shortcuts_enabled;
        let pending_scroll = &self.pending_scroll;
        let mut ctx = Self::make_plugin_context(
            None,
            zoom,
            visible_pages,
            &egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            None,
            pending_scroll,
        );

        for plugin in &mut self.plugins {
            plugin.on_register(&mut ctx);
        }
    }

    fn dispatch_frame_update(&mut self, egui_ctx: &egui::Context) {
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.as_slice();
        let selection_color = self.selection_color;
        let library_shortcuts_enabled = self.library_shortcuts_enabled;
        let pending_scroll = &self.pending_scroll;
        let mut ctx = Self::make_plugin_context(
            document,
            zoom,
            visible_pages,
            egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            None,
            pending_scroll,
        );

        for plugin in &mut self.plugins {
            plugin.on_frame_update(&mut ctx);
        }

        if ctx.repaint_requested() {
            egui_ctx.request_repaint();
        }
    }

    fn current_document_handle(&self) -> Option<Arc<Document>> {
        match &self.state {
            ViewerState::Loaded { document, .. } => Some(Arc::clone(document)),
            ViewerState::Empty | ViewerState::Error(_) => None,
        }
    }

    fn current_context_zoom(&self) -> f32 {
        match self.render.zoom {
            ZoomState::Discrete(zoom) | ZoomState::Custom(zoom) => zoom,
            ZoomState::FitWidth | ZoomState::FitPage => 1.0,
        }
    }

    // justify: the helper mirrors PluginContext's explicit per-dispatch inputs,
    // so keeping the arguments flat makes each call site's viewer state obvious.
    #[allow(clippy::too_many_arguments)]
    fn make_plugin_context<'a>(
        document: Option<&'a Document>,
        zoom: f32,
        visible_pages: &'a [PageIndex],
        egui_ctx: &'a egui::Context,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        page_rect_screen: Option<egui::Rect>,
        pending_scroll: &'a Cell<Option<(PageIndex, PointsRect)>>,
    ) -> PluginContext<'a> {
        PluginContext::new(
            document,
            zoom,
            visible_pages,
            egui_ctx,
            selection_color,
            library_shortcuts_enabled,
            page_rect_screen,
            pending_scroll,
        )
    }

    fn pointer_event(
        ui: &egui::Ui,
        response: &egui::Response,
        page_rect: Rect,
        effective_zoom: f32,
    ) -> Option<PointerEvent> {
        let pointer_pos = if response.hovered() {
            ui.input(|input| input.pointer.hover_pos())
        } else if response.dragged() || response.drag_started() || response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        }?;
        let inside_page_rect = page_rect.contains(pointer_pos);

        let click_count = if response.triple_clicked() {
            3
        } else if response.double_clicked() {
            2
        } else {
            // justify: u8::from avoids clippy::bool_to_int_with_if while keeping
            // the intent clear: 1 when a drag or click starts, 0 otherwise.
            u8::from(response.drag_started() || response.clicked())
        };

        Some(PointerEvent {
            pos_in_page_pt: viewkai_core::PointsPos {
                x: (pointer_pos.x - page_rect.min.x) / effective_zoom,
                y: (pointer_pos.y - page_rect.min.y) / effective_zoom,
            },
            inside_page_rect,
            primary_down: ui.input(|input| input.pointer.primary_down()),
            modifiers: ui.input(|input| input.modifiers),
            click_count,
        })
    }
}

/// Return the canonical crate name.
#[must_use]
pub fn library_name() -> &'static str {
    NAME
}
