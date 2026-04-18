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

pub const NAME: &str = "viewkai";

use egui::{Color32, Rect, Sense, TextureHandle, TextureOptions, Vec2};
use std::sync::Arc;
use viewkai_core::{error::Result, page::PageIndex};
use viewkai_engine::Document;

/// Per-page rendering state.
pub struct PageState {
    /// Page dimensions in PDF points (1 pt = 1/72 inch).
    pub size_pt: Vec2,
    /// Uploaded egui texture, or `None` if not yet rasterized.
    pub texture: Option<TextureHandle>,
    /// Monotonic timestamp of the last rasterization (seconds since epoch).
    pub last_rendered_at: f64,
}

/// Display state for the viewer.
enum ViewerState {
    Empty,
    Loaded {
        document: Arc<Document>,
        pages: Vec<PageState>,
    },
    Error(String),
}

/// An embeddable PDF viewer widget.
///
/// Maintains its own document and texture state. Call [`Self::show`] each frame
/// to render the widget into the provided [`egui::Ui`].
pub struct Viewer {
    state: ViewerState,
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Viewer {
    /// Create a new, empty viewer.
    pub fn new() -> Self {
        Self {
            state: ViewerState::Empty,
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
    /// PDFium engine has not been initialised (call [`viewkai_engine::init()`]
    /// first).
    pub fn load_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
        match Document::from_bytes(bytes) {
            Ok(doc) => {
                let count = doc.page_count();
                let pages = (0..count)
                    .map(|i| {
                        let size = doc
                            .page_size(PageIndex(i))
                            .map(|page| Vec2::new(page.width_pt, page.height_pt))
                            .unwrap_or(Vec2::new(612.0, 792.0));

                        PageState {
                            size_pt: size,
                            texture: None,
                            last_rendered_at: 0.0,
                        }
                    })
                    .collect();

                self.state = ViewerState::Loaded {
                    document: Arc::new(doc),
                    pages,
                };

                Ok(())
            }
            Err(err) => {
                self.state = ViewerState::Error(err.to_string());
                Err(err)
            }
        }
    }

    /// Drop the document and all textures, returning to the empty state.
    pub fn clear(&mut self) {
        self.state = ViewerState::Empty;
    }

    /// Returns the number of pages in the loaded document, or 0 if no document is loaded.
    pub fn page_count(&self) -> usize {
        match &self.state {
            ViewerState::Loaded { pages, .. } => pages.len(),
            _ => 0,
        }
    }

    /// Returns the page size in PDF points for the given index, if loaded.
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
            ViewerState::Error(message) => {
                let message = message.clone();

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
                Self::show_pages(ui, document, pages);
            }
        }

        if should_clear {
            self.clear();
        }
    }

    fn show_pages(ui: &mut egui::Ui, document: &Arc<Document>, pages: &mut [PageState]) {
        const GAP: f32 = 16.0;
        const PLACEHOLDER_FILL: Color32 = Color32::from_gray(220);
        const RENDER_DPI: u32 = 100;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let available_width = ui.available_width();
                let viewport_rect = ui.clip_rect();

                for (idx, page) in pages.iter_mut().enumerate() {
                    let display_size = page.size_pt;
                    let x_offset = ((available_width - display_size.x) / 2.0).max(0.0);
                    let (row_rect, _) = ui.allocate_exact_size(
                        Vec2::new(available_width, display_size.y + GAP),
                        Sense::hover(),
                    );

                    let page_rect = Rect::from_min_size(
                        row_rect.min + Vec2::new(x_offset, 0.0),
                        display_size,
                    );
                    let is_visible = page_rect.intersects(viewport_rect);

                    if is_visible && page.texture.is_none() {
                        if let Ok(raw) = viewkai_engine::render_page(document, PageIndex(idx), RENDER_DPI)
                        {
                            let image = egui::ColorImage::from_rgba_unmultiplied(
                                [raw.width as usize, raw.height as usize],
                                &raw.pixels,
                            );
                            let texture = ui.ctx().load_texture(
                                format!("viewkai/page/{idx}"),
                                image,
                                TextureOptions::LINEAR,
                            );

                            page.texture = Some(texture);
                            page.last_rendered_at = ui.input(|input| input.time);
                        }
                    }

                    match &page.texture {
                        Some(texture) if is_visible => {
                            ui.painter().image(
                                texture.id(),
                                page_rect,
                                Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                        }
                        _ => {
                            ui.painter().rect_filled(page_rect, 0.0, PLACEHOLDER_FILL);

                            if is_visible {
                                egui::Spinner::new().paint_at(ui, page_rect);
                            }
                        }
                    }
                }
            });
    }
}

pub fn library_name() -> &'static str {
    NAME
}
