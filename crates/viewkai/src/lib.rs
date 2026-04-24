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
const NO_VISIBLE_PAGES: &[PageIndex] = &[];

use std::{cell::Cell, collections::HashMap, sync::Arc};
use viewkai_core::{
    Outline, PageSize, PageText, PointsRect, SelectionRange, forward_rotate_rect, rotated_page_size,
};
use viewkai_engine::{Document, error::EngineError};
use viewkai_plugins::PluginRegistry;

pub use viewkai_core::ViewMode;
pub use viewkai_core::outline::{DestPosition, Destination, OutlineNode, OutlineNodeId};
pub use viewkai_core::{PageIndex, PdfPageRotation, RotationDelta};
pub use viewkai_plugins::{
    OutlinePlugin, PluginContext, PointerEvent, SearchPlugin, TextLayerPlugin, ThumbnailPlugin,
    ViewerPlugin,
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
    view_mode: ViewMode,
    current_page_single_mode: Option<PageIndex>,
    current_spread_index: Option<usize>,
    pending_scroll_to_page: Option<usize>,
    plugins: PluginRegistry,
    pending_scroll: Cell<Option<(PageIndex, PointsRect)>>,
    selection_color: Color32,
    library_shortcuts_enabled: bool,
    last_visible_pages: Vec<PageIndex>,
    page_rotations: HashMap<PageIndex, PdfPageRotation>,
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
            view_mode: ViewMode::Continuous,
            current_page_single_mode: None,
            current_spread_index: None,
            pending_scroll_to_page: None,
            plugins: PluginRegistry::new(vec![
                Box::new(TextLayerPlugin::new()),
                Box::new(SearchPlugin::new()),
                Box::new(OutlinePlugin::new()),
                Box::new(ThumbnailPlugin::new()),
            ]),
            pending_scroll: Cell::new(None),
            selection_color: Color32::from_rgba_unmultiplied(70, 120, 210, 96),
            library_shortcuts_enabled: true,
            last_visible_pages: Vec::new(),
            page_rotations: HashMap::new(),
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

    /// Set the active page-layout mode for subsequent renders.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode == mode {
            return;
        }

        let page_count = self.page_count();
        let anchor_page = self.mode_anchor_page();
        self.view_mode = mode;

        match mode {
            ViewMode::Single => {
                self.current_page_single_mode = anchor_page
                    .or_else(|| self.last_visible_pages.first().copied())
                    .and_then(|page| Self::clamp_page_index(page_count, page.0))
                    .or_else(|| Self::clamp_page_index(page_count, 0));
            }
            ViewMode::Spread { cover_separate } => {
                self.current_spread_index = anchor_page
                    .map(|page| Self::spread_index_for_page(page_count, cover_separate, page.0));
            }
            ViewMode::Continuous => {
                self.pending_scroll_to_page = anchor_page.map(|page| page.0);
            }
        }
    }

    /// Return the currently configured page-layout mode.
    #[must_use]
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Scroll the viewer to make page `idx` visible.
    ///
    /// In [`ViewMode::Continuous`] this queues a scroll target that is applied on
    /// the next [`Self::show`] call. In discrete modes it immediately switches the
    /// active page or spread anchor.
    pub fn scroll_to_page(&mut self, idx: usize) {
        match self.view_mode {
            ViewMode::Single => {
                self.current_page_single_mode = Self::clamp_page_index(self.page_count(), idx);
            }
            ViewMode::Spread { cover_separate } => {
                let page_count = self.page_count();
                self.current_spread_index = if page_count == 0 {
                    None
                } else {
                    Some(Self::spread_index_for_page(page_count, cover_separate, idx))
                };
            }
            ViewMode::Continuous => {
                self.pending_scroll_to_page = Some(idx);
            }
        }
    }

    /// Advance to the next discrete page or spread in the current view mode.
    pub fn navigate_next_page(&mut self) {
        let page_count = self.page_count();
        if page_count == 0 {
            return;
        }

        match self.view_mode {
            ViewMode::Single => {
                let current = self.current_page_single_mode.map_or(0, |page| page.0);
                self.current_page_single_mode = Some(PageIndex((current + 1).min(page_count - 1)));
            }
            ViewMode::Spread { cover_separate } => {
                let current = self.current_spread_index.unwrap_or_else(|| {
                    self.mode_anchor_page().map_or(0, |page| {
                        Self::spread_index_for_page(page_count, cover_separate, page.0)
                    })
                });
                self.current_spread_index =
                    Some((current + 1).min(Self::spread_count(page_count, cover_separate) - 1));
            }
            ViewMode::Continuous => {}
        }
    }

    /// Move to the previous discrete page or spread in the current view mode.
    pub fn navigate_prev_page(&mut self) {
        let page_count = self.page_count();
        if page_count == 0 {
            return;
        }

        match self.view_mode {
            ViewMode::Single => {
                let current = self.current_page_single_mode.map_or(0, |page| page.0);
                self.current_page_single_mode = Some(PageIndex(current.saturating_sub(1)));
            }
            ViewMode::Spread { cover_separate } => {
                let current = self.current_spread_index.unwrap_or_else(|| {
                    self.mode_anchor_page().map_or(0, |page| {
                        Self::spread_index_for_page(page_count, cover_separate, page.0)
                    })
                });
                self.current_spread_index = Some(current.saturating_sub(1));
            }
            ViewMode::Continuous => {}
        }
    }

    /// Return the pages visible in the most recent rendered frame.
    #[must_use]
    pub fn visible_pages(&self) -> &[PageIndex] {
        &self.last_visible_pages
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

    /// Returns a shared reference to the built-in outline plugin.
    ///
    /// # Panics
    ///
    /// Panics if the outline plugin is not registered.
    #[must_use]
    pub fn outline(&self) -> &OutlinePlugin {
        match self.plugins.get::<OutlinePlugin>() {
            Some(plugin) => plugin,
            None => panic!("OutlinePlugin is always registered"),
        }
    }

    /// Returns a mutable reference to the built-in outline plugin.
    ///
    /// # Panics
    ///
    /// Panics if the outline plugin is not registered.
    pub fn outline_mut(&mut self) -> &mut OutlinePlugin {
        match self.plugins.get_mut::<OutlinePlugin>() {
            Some(plugin) => plugin,
            None => panic!("OutlinePlugin is always registered"),
        }
    }

    /// Returns a shared reference to the built-in thumbnail plugin.
    ///
    /// # Panics
    ///
    /// Panics if the thumbnail plugin is not registered.
    #[must_use]
    pub fn thumbnails(&self) -> &ThumbnailPlugin {
        match self.plugins.get::<ThumbnailPlugin>() {
            Some(plugin) => plugin,
            None => panic!("ThumbnailPlugin is always registered"),
        }
    }

    /// Returns a mutable reference to the built-in thumbnail plugin.
    ///
    /// # Panics
    ///
    /// Panics if the thumbnail plugin is not registered.
    pub fn thumbnails_mut(&mut self) -> &mut ThumbnailPlugin {
        match self.plugins.get_mut::<ThumbnailPlugin>() {
            Some(plugin) => plugin,
            None => panic!("ThumbnailPlugin is always registered"),
        }
    }

    /// Return a cached page thumbnail texture, queueing rendering when absent.
    pub fn thumbnail_texture(
        &mut self,
        ui: &mut egui::Ui,
        page: PageIndex,
    ) -> Option<egui::TextureHandle> {
        let doc_arc = self.document_arc()?;
        let rotation = self.rotation_of(page);
        self.thumbnails_mut()
            .thumbnail_texture(ui, &doc_arc, page, rotation)
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

    /// Return the zero-based index of the current search match.
    #[must_use]
    pub fn current_match_index(&self) -> Option<usize> {
        self.search().current_match_index()
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

    /// Rotate a single page by one display-time step.
    pub fn rotate_page(&mut self, page: PageIndex, delta: RotationDelta) {
        let current = self.page_rotations.get(&page).copied().unwrap_or_default();
        let new_rotation = current.apply(delta);
        if new_rotation == PdfPageRotation::None {
            self.page_rotations.remove(&page);
        } else {
            self.page_rotations.insert(page, new_rotation);
        }
        self.render.cache.clear();
    }

    /// Rotate all pages by one display-time step.
    pub fn rotate_all(&mut self, delta: RotationDelta) {
        for index in 0..self.page_count() {
            let page = PageIndex(index);
            let current = self.page_rotations.get(&page).copied().unwrap_or_default();
            let new_rotation = current.apply(delta);
            if new_rotation == PdfPageRotation::None {
                self.page_rotations.remove(&page);
            } else {
                self.page_rotations.insert(page, new_rotation);
            }
        }
        self.render.cache.clear();
    }

    /// Return the active display-time rotation for `page`.
    #[must_use]
    pub fn rotation_of(&self, page: PageIndex) -> PdfPageRotation {
        self.page_rotations.get(&page).copied().unwrap_or_default()
    }

    /// Reset all display-time page rotations.
    pub fn reset_rotations(&mut self) {
        self.page_rotations.clear();
        self.render.cache.clear();
    }

    /// Select all text in the loaded document.
    pub fn select_all(&mut self) {
        let egui_ctx = egui::Context::default();
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let visible_pages = self.last_visible_pages.clone();
        let pending_scroll = Cell::new(None);
        let rotations = self.page_rotations.clone();
        let ctx = Self::make_plugin_context(
            document,
            zoom,
            &visible_pages,
            &egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            &rotations,
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
            NO_VISIBLE_PAGES,
            &egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            &self.page_rotations,
            None,
            &self.pending_scroll,
        );
        self.text_layer().selected_text(&ctx)
    }

    /// Return the loaded document handle, if any.
    #[must_use]
    pub fn document_arc(&self) -> Option<Arc<Document>> {
        self.current_document_handle()
    }

    /// Return the loaded document outline, if any.
    #[must_use]
    pub fn outline_document(&self) -> Option<Arc<Outline>> {
        match &self.state {
            ViewerState::Loaded { document, .. } => document.outline().ok(),
            ViewerState::Empty | ViewerState::Error(_) => None,
        }
    }

    /// Queue navigation to an outline destination.
    pub fn goto_destination(&mut self, dest: Destination) {
        self.outline_mut().set_pending_destination(dest);
    }

    /// Copy the selected text to the clipboard.
    pub fn copy_selected_text(&self, egui_ctx: &egui::Context) {
        let document_handle = self.current_document_handle();
        let document = document_handle.as_deref();
        let zoom = self.current_context_zoom();
        let ctx = Self::make_plugin_context(
            document,
            zoom,
            NO_VISIBLE_PAGES,
            egui_ctx,
            self.selection_color,
            self.library_shortcuts_enabled,
            &self.page_rotations,
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
            &self.page_rotations,
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
            &self.page_rotations,
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
                self.current_page_single_mode = None;
                self.current_spread_index = None;
                self.pending_scroll_to_page = None;
                self.pending_scroll.set(None);
                self.last_visible_pages.clear();
                self.page_rotations.clear();
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
        self.current_page_single_mode = None;
        self.current_spread_index = None;
        self.pending_scroll_to_page = None;
        self.pending_scroll.set(None);
        self.last_visible_pages.clear();
        self.page_rotations.clear();
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

        if self.library_shortcuts_enabled {
            let rotate_left = ui.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers {
                        ctrl: true,
                        shift: true,
                        ..egui::Modifiers::NONE
                    },
                    egui::Key::L,
                ))
            });
            let rotate_right = ui.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers {
                        ctrl: true,
                        shift: true,
                        ..egui::Modifiers::NONE
                    },
                    egui::Key::R,
                ))
            });

            if rotate_left {
                self.rotate_all(RotationDelta::CounterClockwise);
            }
            if rotate_right {
                self.rotate_all(RotationDelta::Clockwise);
            }
        }

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
                    self.view_mode,
                    document,
                    pages,
                    &mut self.render.cache,
                    &self.render.visibility,
                    self.render.zoom,
                    &mut self.pending_scroll_to_page,
                    &mut self.current_page_single_mode,
                    &mut self.current_spread_index,
                    &mut self.plugins,
                    &mut self.last_visible_pages,
                    &self.page_rotations,
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
        view_mode: ViewMode,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        visibility: &VisibilityTracker,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
        current_page_single_mode: &mut Option<PageIndex>,
        current_spread_index: &mut Option<usize>,
        plugins: &mut PluginRegistry,
        last_visible_pages: &mut Vec<PageIndex>,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        pending_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
    ) {
        match view_mode {
            ViewMode::Single => Self::show_pages_single(
                ui,
                document,
                pages,
                cache,
                zoom,
                viewer_pending_scroll,
                current_page_single_mode,
                plugins,
                last_visible_pages,
                page_rotations,
                selection_color,
                library_shortcuts_enabled,
                pending_scroll,
            ),
            ViewMode::Continuous => Self::show_pages_continuous(
                ui,
                document,
                pages,
                cache,
                visibility,
                zoom,
                viewer_pending_scroll,
                plugins,
                last_visible_pages,
                page_rotations,
                selection_color,
                library_shortcuts_enabled,
                pending_scroll,
            ),
            ViewMode::Spread { cover_separate } => Self::show_pages_spread(
                ui,
                document,
                pages,
                cache,
                zoom,
                viewer_pending_scroll,
                current_spread_index,
                cover_separate,
                plugins,
                last_visible_pages,
                page_rotations,
                selection_color,
                library_shortcuts_enabled,
                pending_scroll,
            ),
        }
    }

    // justify: page rendering stays snapshot-stable when the existing render inputs
    // and plugin-dispatch state are passed explicitly instead of introducing a new struct.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn show_pages_single(
        ui: &mut egui::Ui,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
        current_page: &mut Option<PageIndex>,
        plugins: &mut PluginRegistry,
        last_visible_pages: &mut Vec<PageIndex>,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        pending_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
    ) {
        if pages.is_empty() {
            last_visible_pages.clear();
            ui.centered_and_justified(|ui| ui.label("No pages in document"));
            return;
        }

        if let Some(target) = viewer_pending_scroll.take() {
            *current_page = Self::clamp_page_index(pages.len(), target);
        }
        let mut pending_target_rect = None;
        if let Some((page, rect_in_page_pt)) = pending_scroll.take() {
            *current_page = Self::clamp_page_index(pages.len(), page.0);
            pending_target_rect = Some(rect_in_page_pt);
        }

        let mut page_idx = current_page.map_or(0, |page| page.0).min(pages.len() - 1);
        if library_shortcuts_enabled {
            let next = ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
            });
            let prev = ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::Space)
            });
            let first = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home));
            let last = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End));

            if next {
                page_idx = (page_idx + 1).min(pages.len() - 1);
            }
            if prev {
                page_idx = page_idx.saturating_sub(1);
            }
            if first {
                page_idx = 0;
            }
            if last {
                page_idx = pages.len() - 1;
            }
        }
        *current_page = Some(PageIndex(page_idx));

        let page = &pages[page_idx];
        let page_rotation = page_rotations
            .get(&PageIndex(page_idx))
            .copied()
            .unwrap_or_default();
        let rotated_size = rotated_page_size(page_state_size(page), page_rotation);
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let viewport_rect = ui.clip_rect();
                let effective_zoom = zoom.effective_zoom(
                    viewport_rect.width(),
                    viewport_rect.height(),
                    rotated_size.width_pt,
                    rotated_size.height_pt,
                );
                let dpi = ZoomState::zoom_to_dpi_bucket(effective_zoom);
                let zoom_bucket = ZoomState::dpi_to_bucket_index(dpi);
                let now = ui.input(|i| i.time);
                let display_size = Vec2::new(
                    rotated_size.width_pt * effective_zoom,
                    rotated_size.height_pt * effective_zoom,
                );
                let content_size = Vec2::new(
                    display_size.x.max(viewport_rect.width()),
                    display_size.y.max(viewport_rect.height()),
                );
                let (content_rect, _) = ui.allocate_exact_size(content_size, Sense::hover());
                let page_rect = Rect::from_min_size(
                    egui::pos2(
                        content_rect.min.x
                            + ((content_rect.width() - display_size.x) / 2.0).max(0.0),
                        content_rect.min.y
                            + ((content_rect.height() - display_size.y) / 2.0).max(0.0),
                    ),
                    display_size,
                );

                if let Some(rect_in_page_pt) = pending_target_rect {
                    ui.scroll_to_rect(
                        Self::rect_in_page(
                            page_rect,
                            rect_in_page_pt,
                            effective_zoom,
                            page_rotation,
                            page_state_size(page),
                        ),
                        Some(egui::Align::Center),
                    );
                }

                last_visible_pages.clear();
                last_visible_pages.push(PageIndex(page_idx));
                Self::render_queued_pages(
                    ui,
                    document,
                    cache,
                    &[page_idx],
                    zoom_bucket,
                    dpi,
                    now,
                    page_rotations,
                );

                let egui_ctx = ui.ctx().clone();
                let mut ctx = Self::make_plugin_context(
                    Some(document.as_ref()),
                    effective_zoom,
                    last_visible_pages.as_slice(),
                    &egui_ctx,
                    selection_color,
                    library_shortcuts_enabled,
                    page_rotations,
                    None,
                    pending_scroll,
                );
                Self::paint_positioned_page(
                    ui,
                    cache,
                    PageIndex(page_idx),
                    page_rect,
                    effective_zoom,
                    zoom_bucket,
                    now,
                    plugins,
                    page_rotation,
                    page_state_size(page),
                    &mut ctx,
                );

                if ctx.repaint_requested() {
                    egui_ctx.request_repaint();
                }
            });
    }

    // justify: page rendering stays snapshot-stable when the existing render inputs
    // and plugin-dispatch state are passed explicitly instead of introducing a new struct.
    #[allow(clippy::too_many_arguments)]
    fn show_pages_continuous(
        ui: &mut egui::Ui,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        visibility: &VisibilityTracker,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
        plugins: &mut PluginRegistry,
        last_visible_pages: &mut Vec<PageIndex>,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
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
                    let first_rotation = page_rotations
                        .get(&PageIndex(0))
                        .copied()
                        .unwrap_or_default();
                    let first_size = rotated_page_size(page_state_size(first), first_rotation);
                    zoom.effective_zoom(
                        available_width,
                        viewport_rect.height(),
                        first_size.width_pt,
                        first_size.height_pt,
                    )
                } else {
                    1.0
                };

                let dpi = ZoomState::zoom_to_dpi_bucket(effective_zoom);
                let zoom_bucket = ZoomState::dpi_to_bucket_index(dpi);

                let (page_tops, page_heights) =
                    Self::compute_page_layout(pages, effective_zoom, page_rotations);
                Self::handle_pending_scroll(
                    ui,
                    viewer_pending_scroll,
                    pending_scroll,
                    &page_tops,
                    pages,
                    page_rotations,
                    effective_zoom,
                    available_width,
                );

                let scroll_offset = ui.clip_rect().min.y - ui.min_rect().min.y;
                let vis_set = Self::visible_pages_in_viewport(
                    visibility,
                    viewport_rect,
                    scroll_offset,
                    &page_tops,
                    &page_heights,
                );
                last_visible_pages.clear();
                last_visible_pages.extend(vis_set.pages.iter().copied());

                let center_y = scroll_offset + viewport_rect.height() / 2.0;
                let to_render = Self::prioritize_renders(&vis_set, &page_tops, center_y);
                Self::render_queued_pages(
                    ui,
                    document,
                    cache,
                    &to_render,
                    zoom_bucket,
                    dpi,
                    now,
                    page_rotations,
                );

                let egui_ctx = ui.ctx().clone();
                let mut ctx = Self::make_plugin_context(
                    Some(document.as_ref()),
                    effective_zoom,
                    last_visible_pages.as_slice(),
                    &egui_ctx,
                    selection_color,
                    library_shortcuts_enabled,
                    page_rotations,
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
                    page_rotations,
                    &mut ctx,
                );

                if ctx.repaint_requested() {
                    egui_ctx.request_repaint();
                }
            });
    }

    // justify: page rendering stays snapshot-stable when the existing render inputs
    // and plugin-dispatch state are passed explicitly instead of introducing a new struct.
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::similar_names
    )]
    fn show_pages_spread(
        ui: &mut egui::Ui,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
        current_spread_index: &mut Option<usize>,
        cover_separate: bool,
        plugins: &mut PluginRegistry,
        last_visible_pages: &mut Vec<PageIndex>,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
        selection_color: Color32,
        library_shortcuts_enabled: bool,
        pending_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
    ) {
        if pages.is_empty() {
            last_visible_pages.clear();
            ui.centered_and_justified(|ui| ui.label("No pages in document"));
            return;
        }

        if let Some(target) = viewer_pending_scroll.take() {
            *current_spread_index = Some(Self::spread_index_for_page(
                pages.len(),
                cover_separate,
                target,
            ));
            pending_scroll.set(None);
        }
        let mut pending_target = None;
        if let Some((page, rect_in_page_pt)) = pending_scroll.take() {
            *current_spread_index = Some(Self::spread_index_for_page(
                pages.len(),
                cover_separate,
                page.0,
            ));
            pending_target = Some((page, rect_in_page_pt));
        }

        let spread_count = Self::spread_count(pages.len(), cover_separate);
        let mut spread_idx = current_spread_index
            .unwrap_or(0)
            .min(spread_count.saturating_sub(1));
        if library_shortcuts_enabled {
            let prev = ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::ArrowLeft));
            let next =
                ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::ArrowRight));

            if prev {
                spread_idx = spread_idx.saturating_sub(1);
            }
            if next {
                spread_idx = (spread_idx + 1).min(spread_count.saturating_sub(1));
            }
        }
        *current_spread_index = Some(spread_idx);

        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let (left_idx, right_idx) =
                    Self::spread_pages(pages.len(), cover_separate, spread_idx);
                let left_page = &pages[left_idx];
                let right_page = right_idx.and_then(|idx| pages.get(idx));
                let left_rotation = page_rotations
                    .get(&PageIndex(left_idx))
                    .copied()
                    .unwrap_or_default();
                let left_rotated = rotated_page_size(page_state_size(left_page), left_rotation);
                let right_rotated = right_idx.and_then(|idx| {
                    pages.get(idx).map(|page| {
                        let rotation = page_rotations
                            .get(&PageIndex(idx))
                            .copied()
                            .unwrap_or_default();
                        rotated_page_size(page_state_size(page), rotation)
                    })
                });
                let spread_width_pt = left_rotated.width_pt
                    + right_rotated.map_or(0.0, |page| page.width_pt)
                    + if right_page.is_some() { 8.0 } else { 0.0 };
                let spread_height_pt = right_rotated.map_or(left_rotated.height_pt, |page| {
                    left_rotated.height_pt.max(page.height_pt)
                });

                let viewport_rect = ui.clip_rect();
                let effective_zoom = zoom.effective_zoom(
                    viewport_rect.width(),
                    viewport_rect.height(),
                    spread_width_pt,
                    spread_height_pt,
                );
                let dpi = ZoomState::zoom_to_dpi_bucket(effective_zoom);
                let zoom_bucket = ZoomState::dpi_to_bucket_index(dpi);
                let now = ui.input(|i| i.time);

                let left_size = Vec2::new(
                    left_rotated.width_pt * effective_zoom,
                    left_rotated.height_pt * effective_zoom,
                );
                let right_size = right_rotated.map(|page| {
                    Vec2::new(
                        page.width_pt * effective_zoom,
                        page.height_pt * effective_zoom,
                    )
                });
                let spread_width_px = left_size.x
                    + right_size.map_or(0.0, |size| size.x)
                    + if right_size.is_some() { 8.0 } else { 0.0 };
                let spread_height_px =
                    right_size.map_or(left_size.y, |size| left_size.y.max(size.y));
                let content_size = Vec2::new(
                    spread_width_px.max(viewport_rect.width()),
                    spread_height_px.max(viewport_rect.height()),
                );
                let (content_rect, _) = ui.allocate_exact_size(content_size, Sense::hover());
                let spread_origin = egui::pos2(
                    content_rect.min.x + ((content_rect.width() - spread_width_px) / 2.0).max(0.0),
                    content_rect.min.y
                        + ((content_rect.height() - spread_height_px) / 2.0).max(0.0),
                );
                let left_rect = Rect::from_min_size(
                    egui::pos2(
                        spread_origin.x,
                        spread_origin.y + ((spread_height_px - left_size.y) / 2.0).max(0.0),
                    ),
                    left_size,
                );
                let right_rect = right_size.map(|size| {
                    Rect::from_min_size(
                        egui::pos2(
                            spread_origin.x + left_size.x + 8.0,
                            spread_origin.y + ((spread_height_px - size.y) / 2.0).max(0.0),
                        ),
                        size,
                    )
                });

                if let Some((page, rect_in_page_pt)) = pending_target {
                    let target_rect = if page.0 == left_idx {
                        Some(Self::rect_in_page(
                            left_rect,
                            rect_in_page_pt,
                            effective_zoom,
                            left_rotation,
                            page_state_size(left_page),
                        ))
                    } else if let Some((right_idx, right_rect)) = right_idx.zip(right_rect) {
                        (page.0 == right_idx).then(|| {
                            let page = &pages[right_idx];
                            let rotation = page_rotations
                                .get(&PageIndex(right_idx))
                                .copied()
                                .unwrap_or_default();
                            Self::rect_in_page(
                                right_rect,
                                rect_in_page_pt,
                                effective_zoom,
                                rotation,
                                page_state_size(page),
                            )
                        })
                    } else {
                        None
                    };
                    if let Some(target_rect) = target_rect {
                        ui.scroll_to_rect(target_rect, Some(egui::Align::Center));
                    }
                }

                let mut to_render = vec![left_idx];
                if let Some(right_idx) = right_idx {
                    to_render.push(right_idx);
                }
                last_visible_pages.clear();
                last_visible_pages.push(PageIndex(left_idx));
                if let Some(right_idx) = right_idx {
                    last_visible_pages.push(PageIndex(right_idx));
                }
                Self::render_queued_pages(
                    ui,
                    document,
                    cache,
                    &to_render,
                    zoom_bucket,
                    dpi,
                    now,
                    page_rotations,
                );

                let egui_ctx = ui.ctx().clone();
                let mut ctx = Self::make_plugin_context(
                    Some(document.as_ref()),
                    effective_zoom,
                    last_visible_pages.as_slice(),
                    &egui_ctx,
                    selection_color,
                    library_shortcuts_enabled,
                    page_rotations,
                    None,
                    pending_scroll,
                );
                Self::paint_positioned_page(
                    ui,
                    cache,
                    PageIndex(left_idx),
                    left_rect,
                    effective_zoom,
                    zoom_bucket,
                    now,
                    plugins,
                    left_rotation,
                    page_state_size(left_page),
                    &mut ctx,
                );
                if let Some((right_idx, right_rect)) = right_idx.zip(right_rect) {
                    let right_page = &pages[right_idx];
                    let right_rotation = page_rotations
                        .get(&PageIndex(right_idx))
                        .copied()
                        .unwrap_or_default();
                    Self::paint_positioned_page(
                        ui,
                        cache,
                        PageIndex(right_idx),
                        right_rect,
                        effective_zoom,
                        zoom_bucket,
                        now,
                        plugins,
                        right_rotation,
                        page_state_size(right_page),
                        &mut ctx,
                    );
                }

                if ctx.repaint_requested() {
                    egui_ctx.request_repaint();
                }
            });
    }

    fn compute_page_layout(
        pages: &[PageState],
        effective_zoom: f32,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
    ) -> (Vec<f32>, Vec<f32>) {
        const GAP: f32 = 16.0;

        let mut page_tops = Vec::with_capacity(pages.len());
        let mut cumulative_y = 0.0_f32;
        for (idx, page) in pages.iter().enumerate() {
            page_tops.push(cumulative_y);
            let rotation = page_rotations
                .get(&PageIndex(idx))
                .copied()
                .unwrap_or_default();
            let page_size = rotated_page_size(page_state_size(page), rotation);
            cumulative_y += page_size.height_pt * effective_zoom + GAP;
        }

        let page_heights = pages
            .iter()
            .enumerate()
            .map(|(idx, page)| {
                let rotation = page_rotations
                    .get(&PageIndex(idx))
                    .copied()
                    .unwrap_or_default();
                rotated_page_size(page_state_size(page), rotation).height_pt * effective_zoom
            })
            .collect();

        (page_tops, page_heights)
    }

    fn compute_page_viewport_rect(
        page_idx: usize,
        effective_zoom: f32,
        pages: &[PageState],
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
        available_width: f32,
    ) -> Rect {
        let (page_tops, _) = Self::compute_page_layout(pages, effective_zoom, page_rotations);
        let page = pages[page_idx];
        let rotation = page_rotations
            .get(&PageIndex(page_idx))
            .copied()
            .unwrap_or_default();
        let rotated_size = rotated_page_size(page_state_size(&page), rotation);
        let display_size = Vec2::new(
            rotated_size.width_pt * effective_zoom,
            rotated_size.height_pt * effective_zoom,
        );
        let x_offset = ((available_width - display_size.x) / 2.0).max(0.0);

        Rect::from_min_size(egui::pos2(x_offset, page_tops[page_idx]), display_size)
    }

    fn visible_pages_in_viewport(
        visibility: &VisibilityTracker,
        viewport_rect: Rect,
        scroll_offset: f32,
        page_tops: &[f32],
        page_heights: &[f32],
    ) -> VisibleSet {
        visibility.compute(
            scroll_offset.max(0.0),
            viewport_rect.height(),
            page_tops,
            page_heights,
        )
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

    #[allow(clippy::too_many_arguments)]
    fn render_queued_pages(
        ui: &egui::Ui,
        document: &Arc<Document>,
        cache: &mut TextureCache,
        to_render: &[usize],
        zoom_bucket: u8,
        dpi: u32,
        now: f64,
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
    ) {
        for &idx in to_render {
            let page_index = PageIndex(idx);
            let rotation = page_rotations.get(&page_index).copied().unwrap_or_default();
            let key = CacheKey {
                page_idx: page_index,
                zoom_bucket,
                rotation,
            };

            if cache.get(&key, now).is_none()
                && let Ok(raw) = viewkai_engine::render_page(document, page_index, dpi, rotation)
            {
                let byte_size = raw.pixels.len();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raw.width as usize, raw.height as usize],
                    &raw.pixels,
                );
                let handle = ui.ctx().load_texture(
                    format!("viewkai/page/{idx}/dpi{dpi}/rot{}", rotation.as_degrees()),
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
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
        plugin_ctx: &mut PluginContext<'_>,
    ) {
        const GAP: f32 = 16.0;
        const PLACEHOLDER_FILL: Color32 = Color32::from_gray(220);

        for (idx, page) in pages.iter().enumerate() {
            let page_index = PageIndex(idx);
            let rotation = page_rotations.get(&page_index).copied().unwrap_or_default();
            let rotated_size = rotated_page_size(page_state_size(page), rotation);
            let display_size = Vec2::new(
                rotated_size.width_pt * effective_zoom,
                rotated_size.height_pt * effective_zoom,
            );
            let x_offset = ((available_width - display_size.x) / 2.0).max(0.0);
            let (row_rect, response) = ui.allocate_exact_size(
                Vec2::new(available_width, display_size.y + GAP),
                Sense::click_and_drag(),
            );
            let page_rect =
                Rect::from_min_size(row_rect.min + Vec2::new(x_offset, 0.0), display_size);

            let key = CacheKey {
                page_idx: page_index,
                zoom_bucket,
                rotation,
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
                        page_idx: page_index,
                        zoom_bucket: bucket,
                        rotation,
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

    #[allow(clippy::too_many_arguments)]
    fn paint_positioned_page(
        ui: &mut egui::Ui,
        cache: &mut TextureCache,
        page_index: PageIndex,
        page_rect: Rect,
        effective_zoom: f32,
        zoom_bucket: u8,
        now: f64,
        plugins: &mut PluginRegistry,
        rotation: PdfPageRotation,
        _page_size: PageSize,
        plugin_ctx: &mut PluginContext<'_>,
    ) {
        const PLACEHOLDER_FILL: Color32 = Color32::from_gray(220);

        let key = CacheKey {
            page_idx: page_index,
            zoom_bucket,
            rotation,
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
                    page_idx: page_index,
                    zoom_bucket: bucket,
                    rotation,
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
                egui::Spinner::new().paint_at(ui, page_rect);
            }
        }

        let response = ui.interact(
            page_rect,
            ui.id().with(("viewkai-positioned-page", page_index.0)),
            Sense::click_and_drag(),
        );

        if let Some(pointer_event) = Self::pointer_event(ui, &response, page_rect, effective_zoom) {
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

    fn clamp_page_index(page_count: usize, idx: usize) -> Option<PageIndex> {
        if page_count == 0 {
            None
        } else {
            Some(PageIndex(idx.min(page_count - 1)))
        }
    }

    fn rect_in_page(
        page_rect: Rect,
        rect_in_page_pt: PointsRect,
        effective_zoom: f32,
        rotation: PdfPageRotation,
        page_size: PageSize,
    ) -> Rect {
        let rect_in_page_pt = forward_rotate_rect(rect_in_page_pt, rotation, page_size);
        Rect::from_min_size(
            egui::pos2(
                page_rect.min.x + rect_in_page_pt.x * effective_zoom,
                page_rect.min.y + rect_in_page_pt.y * effective_zoom,
            ),
            Vec2::new(
                rect_in_page_pt.width.max(1.0) * effective_zoom,
                rect_in_page_pt.height.max(1.0) * effective_zoom,
            ),
        )
    }

    fn mode_anchor_page(&self) -> Option<PageIndex> {
        let page_count = self.page_count();
        if page_count == 0 {
            return None;
        }

        match self.view_mode {
            ViewMode::Single => self
                .current_page_single_mode
                .or_else(|| self.last_visible_pages.first().copied())
                .or(Some(PageIndex(0))),
            ViewMode::Spread { cover_separate } => {
                let spread_idx = self.current_spread_index.unwrap_or_else(|| {
                    self.last_visible_pages.first().map_or(0, |page| {
                        Self::spread_index_for_page(page_count, cover_separate, page.0)
                    })
                });
                Some(PageIndex(
                    Self::spread_pages(page_count, cover_separate, spread_idx).0,
                ))
            }
            ViewMode::Continuous => self
                .last_visible_pages
                .first()
                .copied()
                .or(Some(PageIndex(0))),
        }
    }

    fn spread_count(page_count: usize, cover_separate: bool) -> usize {
        if page_count == 0 {
            0
        } else if cover_separate {
            1 + (page_count.saturating_sub(1)).div_ceil(2)
        } else {
            page_count.div_ceil(2)
        }
    }

    fn spread_pages(
        page_count: usize,
        cover_separate: bool,
        spread_idx: usize,
    ) -> (usize, Option<usize>) {
        if page_count == 0 {
            return (0, None);
        }

        if cover_separate {
            if spread_idx == 0 {
                return (0, None);
            }
            let offset = 1 + (spread_idx - 1) * 2;
            let left = offset.min(page_count - 1);
            (
                left,
                if left + 1 < page_count {
                    Some(left + 1)
                } else {
                    None
                },
            )
        } else {
            let offset = (spread_idx * 2).min(page_count - 1);
            (
                offset,
                if offset + 1 < page_count {
                    Some(offset + 1)
                } else {
                    None
                },
            )
        }
    }

    fn spread_index_for_page(page_count: usize, cover_separate: bool, page_idx: usize) -> usize {
        if page_count == 0 {
            0
        } else {
            let page_idx = page_idx.min(page_count - 1);
            if cover_separate {
                if page_idx == 0 {
                    0
                } else {
                    1 + (page_idx - 1) / 2
                }
            } else {
                page_idx / 2
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_pending_scroll(
        ui: &mut egui::Ui,
        pending_scroll_to_page: &mut Option<usize>,
        pending_plugin_scroll: &Cell<Option<(PageIndex, PointsRect)>>,
        page_tops: &[f32],
        pages: &[PageState],
        page_rotations: &HashMap<PageIndex, PdfPageRotation>,
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
            && pages.get(page.0).is_some()
        {
            let page_rect = Self::compute_page_viewport_rect(
                page.0,
                effective_zoom,
                pages,
                page_rotations,
                available_width,
            );
            let page_size = page_state_size(&pages[page.0]);
            let rotation = page_rotations.get(&page).copied().unwrap_or_default();
            let target_rect = Self::rect_in_page(
                page_rect,
                rect_in_page_pt,
                effective_zoom,
                rotation,
                page_size,
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
            &self.page_rotations,
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
            &self.page_rotations,
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
        rotations: &'a HashMap<PageIndex, PdfPageRotation>,
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
            rotations,
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

#[allow(clippy::trivially_copy_pass_by_ref)]
fn page_state_size(page: &PageState) -> PageSize {
    PageSize {
        width_pt: page.size_pt.x,
        height_pt: page.size_pt.y,
    }
}

/// Return the canonical crate name.
#[must_use]
pub fn library_name() -> &'static str {
    NAME
}

/// Initialise the PDF engine.
///
/// Must be called once before loading any document. Safe to call multiple times.
///
/// # Errors
/// Returns an error if the pdfium library cannot be loaded.
pub fn init() -> viewkai_core::Result<()> {
    viewkai_engine::init().map_err(|err| viewkai_core::Error::Engine(err.to_string()))
}
