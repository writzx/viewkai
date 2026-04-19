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
use std::sync::Arc;
use viewkai_core::page::PageIndex;
use viewkai_engine::{Document, error::EngineError};

/// Per-page rendering state.
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
        Self {
            state: ViewerState::Empty,
            render: RenderState::new(),
            pending_scroll_to_page: None,
        }
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
                            .map(|page| Vec2::new(page.width_pt, page.height_pt))
                            .unwrap_or(Vec2::new(612.0, 792.0));

                        PageState { size_pt: size }
                    })
                    .collect();

                self.render.cache.clear();
                self.pending_scroll_to_page = None;
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

    /// Render the viewer into the given [`egui::Ui`].
    ///
    /// - **Empty**: shows "No document loaded".
    /// - **Error**: shows the error message and a "Retry" button.
    /// - **Loaded**: shows all pages in a vertical scroll area with lazy
    ///   rasterization.
    pub fn show(&mut self, ui: &mut egui::Ui) {
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
                );
            }
        }

        if should_clear {
            self.clear();
        }
    }

    fn show_pages(
        ui: &mut egui::Ui,
        document: &Arc<Document>,
        pages: &[PageState],
        cache: &mut TextureCache,
        visibility: &VisibilityTracker,
        zoom: ZoomState,
        viewer_pending_scroll: &mut Option<usize>,
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
                Self::handle_pending_scroll(ui, viewer_pending_scroll, &page_tops);

                let scroll_offset = ui.clip_rect().min.y - ui.min_rect().min.y;
                let vis_set = visibility.compute(scroll_offset.max(0.0), viewport_rect.height(), &page_tops, &page_heights);

                let center_y = scroll_offset + viewport_rect.height() / 2.0;
                let to_render = Self::prioritize_renders(&vis_set, &page_tops, center_y);
                Self::render_queued_pages(ui, document, cache, &to_render, zoom_bucket, dpi, now);

                Self::paint_pages(ui, pages, cache, &vis_set, effective_zoom, zoom_bucket, available_width, now);
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

    fn prioritize_renders(
        vis_set: &VisibleSet,
        page_tops: &[f32],
        center_y: f32,
    ) -> Vec<usize> {
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
    ) {
        const GAP: f32 = 16.0;
        const PLACEHOLDER_FILL: Color32 = Color32::from_gray(220);

        for (idx, page) in pages.iter().enumerate() {
            let display_size = Vec2::new(
                page.size_pt.x * effective_zoom,
                page.size_pt.y * effective_zoom,
            );
            let x_offset = ((available_width - display_size.x) / 2.0).max(0.0);
            let (row_rect, _) = ui.allocate_exact_size(
                Vec2::new(available_width, display_size.y + GAP),
                Sense::hover(),
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
        }
    }

    fn handle_pending_scroll(
        ui: &mut egui::Ui,
        pending_scroll: &mut Option<usize>,
        page_tops: &[f32],
    ) {
        if let Some(target_idx) = pending_scroll.take()
            && let Some(&top) = page_tops.get(target_idx)
        {
            ui.scroll_to_rect(
                Rect::from_min_size(egui::pos2(0.0, top), Vec2::new(1.0, 1.0)),
                Some(egui::Align::TOP),
            );
        }
    }
}

/// Return the canonical crate name.
#[must_use]
pub fn library_name() -> &'static str {
    NAME
}
