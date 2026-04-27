//! Thumbnail sidebar plugin.

use std::collections::HashMap;

use egui::{Color32, TextureHandle, TextureOptions, Ui, Vec2};
use viewkai_core::{PageIndex, PdfPageRotation, PointsRect};
use viewkai_engine::Document;

use crate::{PluginContext, ViewerPlugin, sealed};

const DEFAULT_THUMBNAIL_BUDGET: usize = 64 * 1024 * 1024;
const THUMBNAILS_PER_FRAME: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ThumbnailCacheKey {
    page: PageIndex,
    rotation: PdfPageRotation,
}

struct CacheEntry {
    texture: TextureHandle,
    byte_size: usize,
    last_accessed_frame: u64,
}

struct ThumbnailCache {
    entries: HashMap<ThumbnailCacheKey, CacheEntry>,
    total_bytes: usize,
    budget_bytes: usize,
    frame_counter: u64,
}

impl ThumbnailCache {
    fn new(budget_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            budget_bytes,
            frame_counter: 0,
        }
    }

    fn get(&mut self, key: ThumbnailCacheKey) -> Option<TextureHandle> {
        let entry = self.entries.get_mut(&key)?;
        entry.last_accessed_frame = self.frame_counter;
        Some(entry.texture.clone())
    }

    fn insert(&mut self, key: ThumbnailCacheKey, texture: TextureHandle, byte_size: usize) {
        if let Some(old) = self.entries.remove(&key) {
            self.total_bytes -= old.byte_size;
        }

        if byte_size > self.budget_bytes {
            return;
        }

        self.entries.insert(
            key,
            CacheEntry {
                texture,
                byte_size,
                last_accessed_frame: self.frame_counter,
            },
        );
        self.total_bytes += byte_size;
        self.evict_to_budget();
    }

    fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    fn set_budget(&mut self, bytes: usize) {
        self.budget_bytes = bytes;
        self.evict_to_budget();
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }

    fn evict_to_budget(&mut self) {
        while self.total_bytes > self.budget_bytes {
            let Some(lru_key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed_frame)
                .map(|(key, _)| *key)
            else {
                break;
            };

            let removed = self
                .entries
                .remove(&lru_key)
                .expect("lru page came from cache entries");
            self.total_bytes -= removed.byte_size;
        }
    }

    fn tick_frame(&mut self) {
        self.frame_counter = self.frame_counter.saturating_add(1);
    }
}

/// Plugin that renders a page-thumbnail sidebar and caches textures separately
/// from the main page texture cache.
pub struct ThumbnailPlugin {
    visible: bool,
    cache: ThumbnailCache,
    pending_pages: Vec<PageIndex>,
    thumbnail_width: u32,
    pending_click_page: Option<PageIndex>,
    active_page: Option<usize>,
    document_identity: Option<usize>,
}

impl ThumbnailPlugin {
    /// Default cache budget for thumbnail textures.
    pub const DEFAULT_CACHE_BUDGET: usize = DEFAULT_THUMBNAIL_BUDGET;

    /// Create a new thumbnail plugin.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached thumbnail texture for a page or queue rendering.
    pub fn thumbnail_texture(
        &mut self,
        ui: &mut Ui,
        document: &Document,
        page: PageIndex,
        rotation: PdfPageRotation,
    ) -> Option<TextureHandle> {
        self.sync_document(document);
        let key = ThumbnailCacheKey { page, rotation };

        if let Some(handle) = self.cache.get(key) {
            return Some(handle);
        }

        if !self.pending_pages.contains(&page) {
            self.pending_pages.push(page);
            ui.ctx().request_repaint();
        }

        None
    }

    /// Render the thumbnail sidebar panel.
    pub fn render_panel(&mut self, ui: &mut Ui, document: Option<&Document>) {
        let Some(doc) = document else {
            ui.label("No document loaded");
            return;
        };

        self.sync_document(doc);

        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                // Reserve gutter space so scrollbar doesn't overlap thumbnails.
                // egui scrollbar is ~8px + margins; 12px is a safe conservative gutter.
                ui.set_width((ui.available_width() - 12.0).max(1.0));
                for page_idx in 0..doc.page_count() {
                    let page = PageIndex(page_idx);
                    let texture = self.thumbnail_texture(ui, doc, page, PdfPageRotation::None);
                    let is_active = self.active_page == Some(page_idx);
                    let preview_height = self.preview_height_for(doc, page);
                    let preview_size = Vec2::new(self.thumbnail_width as f32, preview_height);
                    let frame = egui::Frame::new()
                        .fill(if is_active {
                            ui.visuals().selection.bg_fill.gamma_multiply(0.2)
                        } else {
                            ui.visuals().widgets.inactive.bg_fill
                        })
                        .stroke(if is_active {
                            ui.visuals().selection.stroke
                        } else {
                            ui.visuals().widgets.noninteractive.bg_stroke
                        })
                        .inner_margin(egui::Margin::same(6))
                        .corner_radius(6.0);
                    let inner = frame.show(ui, |ui| {
                        let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());

                        if let Some(texture) = texture {
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                        } else {
                            ui.painter().rect_filled(rect, 4.0, Color32::from_gray(210));
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Loading…",
                                egui::TextStyle::Body.resolve(ui.style()),
                                Color32::from_gray(60),
                            );
                        }
                    });
                    let response = ui.interact(
                        inner.response.rect,
                        ui.make_persistent_id(("thumbnail", page_idx)),
                        egui::Sense::click(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            format!("Page {}", page_idx + 1),
                        )
                    });

                    if response.hovered() && !is_active {
                        ui.painter().rect_stroke(
                            response.rect,
                            6.0,
                            ui.visuals().widgets.hovered.bg_stroke,
                            egui::StrokeKind::Outside,
                        );
                    }

                    if response.clicked() {
                        self.pending_click_page = Some(page);
                        ui.ctx().request_repaint();
                    }

                    ui.add_space(8.0);
                }
            });
    }

    /// Set the thumbnail cache budget in bytes.
    pub fn set_cache_budget(&mut self, bytes: usize) {
        self.cache.set_budget(bytes);
    }

    /// Return the current thumbnail cache usage in bytes.
    #[must_use]
    pub fn cache_bytes(&self) -> usize {
        self.cache.total_bytes()
    }

    /// Return whether the thumbnail sidebar is visible.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Show or hide the thumbnail sidebar.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Return the currently queued click target, if any.
    #[must_use]
    pub fn pending_click_page(&self) -> Option<PageIndex> {
        self.pending_click_page
    }

    fn preview_height_for(&self, document: &Document, page: PageIndex) -> f32 {
        document
            .page_size(page)
            .map_or(self.thumbnail_width as f32 * 1.4, |size| {
                let aspect = if size.width_pt > 0.0 {
                    size.height_pt / size.width_pt
                } else {
                    1.4
                };
                (self.thumbnail_width as f32 * aspect).max(1.0)
            })
    }

    fn reset_document_state(&mut self) {
        self.cache.clear();
        self.pending_pages.clear();
        self.pending_click_page = None;
        self.active_page = None;
    }

    fn sync_document(&mut self, document: &Document) {
        let identity = std::ptr::from_ref::<Document>(document) as usize;
        if self.document_identity != Some(identity) {
            self.reset_document_state();
            self.document_identity = Some(identity);
        }
    }

    fn update_active_page(&mut self, ctx: &PluginContext<'_>) -> bool {
        let next_active_page = ctx.visible_pages.first().map(|page| page.0);
        if self.active_page == next_active_page {
            return false;
        }
        self.active_page = next_active_page;
        true
    }
}

impl Default for ThumbnailPlugin {
    fn default() -> Self {
        Self {
            visible: false,
            cache: ThumbnailCache::new(DEFAULT_THUMBNAIL_BUDGET),
            pending_pages: Vec::new(),
            thumbnail_width: 120,
            pending_click_page: None,
            active_page: None,
            document_identity: None,
        }
    }
}

impl sealed::Sealed for ThumbnailPlugin {}

impl ViewerPlugin for ThumbnailPlugin {
    fn id(&self) -> &'static str {
        "viewkai.thumbnail"
    }

    fn show_toolbar(&mut self, _ui: &mut Ui, _ctx: &mut PluginContext<'_>) {}

    fn on_frame_update(&mut self, ctx: &mut PluginContext<'_>) {
        self.cache.tick_frame();
        let active_page_changed = self.update_active_page(ctx);

        let Some(document) = ctx.document else {
            if self.document_identity.take().is_some() {
                self.reset_document_state();
            }
            return;
        };

        self.sync_document(document);

        let pending = self
            .pending_pages
            .drain(..self.pending_pages.len().min(THUMBNAILS_PER_FRAME))
            .collect::<Vec<_>>();

        for page in pending {
            let rotation = ctx.rotation_of(page);
            let key = ThumbnailCacheKey { page, rotation };

            if self.cache.get(key).is_some() {
                continue;
            }

            if let Ok(raw) =
                viewkai_engine::render_thumbnail(document, page, self.thumbnail_width, rotation)
            {
                let byte_size = raw.pixels.len();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raw.width as usize, raw.height as usize],
                    &raw.pixels,
                );
                let handle = ctx.egui_ctx.load_texture(
                    format!(
                        "viewkai/thumbnail/{:p}/{}/{}",
                        document,
                        page.0,
                        rotation.as_degrees()
                    ),
                    image,
                    TextureOptions::LINEAR,
                );
                self.cache.insert(key, handle, byte_size);
            }
        }

        if let Some(page) = self.pending_click_page.take() {
            ctx.request_scroll_to(
                page,
                PointsRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                },
            );
            ctx.request_repaint();
        }

        if active_page_changed || !self.pending_pages.is_empty() {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::HashMap};

    use egui::Color32;

    use super::*;

    fn plugin_ctx<'a>(
        egui_ctx: &'a egui::Context,
        pending_scroll: &'a Cell<Option<(PageIndex, PointsRect)>>,
        visible_pages: &'a [PageIndex],
        rotations: &'a HashMap<PageIndex, PdfPageRotation>,
    ) -> PluginContext<'a> {
        PluginContext::new(
            None,
            1.0,
            visible_pages,
            egui_ctx,
            Color32::WHITE,
            true,
            rotations,
            None,
            pending_scroll,
        )
    }

    #[test]
    fn active_page_updates_only_on_transition() {
        let egui_ctx = egui::Context::default();
        let pending_scroll = Cell::new(None);
        let rotations = HashMap::new();
        let mut plugin = ThumbnailPlugin::new();

        let first = [PageIndex(2), PageIndex(3)];
        assert!(plugin.update_active_page(&plugin_ctx(
            &egui_ctx,
            &pending_scroll,
            &first,
            &rotations
        )));
        assert_eq!(plugin.active_page, Some(2));

        assert!(!plugin.update_active_page(&plugin_ctx(
            &egui_ctx,
            &pending_scroll,
            &first,
            &rotations
        )));
        assert_eq!(plugin.active_page, Some(2));

        let second = [PageIndex(5)];
        assert!(plugin.update_active_page(&plugin_ctx(
            &egui_ctx,
            &pending_scroll,
            &second,
            &rotations
        )));
        assert_eq!(plugin.active_page, Some(5));
    }

    #[test]
    fn active_page_clears_when_viewport_is_empty() {
        let egui_ctx = egui::Context::default();
        let pending_scroll = Cell::new(None);
        let rotations = HashMap::new();
        let mut plugin = ThumbnailPlugin::new();

        let visible = [PageIndex(1)];
        assert!(plugin.update_active_page(&plugin_ctx(
            &egui_ctx,
            &pending_scroll,
            &visible,
            &rotations
        )));
        assert_eq!(plugin.active_page, Some(1));

        assert!(plugin.update_active_page(&plugin_ctx(
            &egui_ctx,
            &pending_scroll,
            &[],
            &rotations
        )));
        assert_eq!(plugin.active_page, None);
    }
}
