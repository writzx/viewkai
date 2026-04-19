//! Text-layer plugin: per-character bbox extraction, word grouping, debug overlay.

use egui::{Color32, Stroke, Ui};
use viewkai_core::{CharIndex, PageIndex, PointsPos, PointsRect, SelectionRange};

use crate::{PluginContext, ViewerPlugin, sealed::Sealed};

/// Plugin that provides text extraction, hit-testing, and a debug overlay.
///
/// In Phase A this plugin extracts per-character bboxes, groups them into
/// words/lines, and renders an optional debug overlay showing word bounding
/// boxes. Selection and clipboard copy are added in Phase B.
#[derive(Debug)]
pub struct TextLayerPlugin {
    debug: bool,
    selection: Option<SelectionRange>,
    selection_anchor: Option<CharIndex>,
}

// justify: Phase B requires an explicit `Default` impl to keep the initial
// selection state spelled out next to the added selection fields.
#[allow(clippy::derivable_impls)]
impl Default for TextLayerPlugin {
    fn default() -> Self {
        Self {
            debug: false,
            selection: None,
            selection_anchor: None,
        }
    }
}

impl TextLayerPlugin {
    /// Create a new text-layer plugin instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether the debug overlay is enabled.
    #[must_use]
    pub fn debug(&self) -> bool {
        self.debug
    }

    /// Enable or disable the debug overlay (word bounding boxes).
    pub fn set_debug(&mut self, enabled: bool) {
        self.debug = enabled;
    }

    /// Return the current selection, if any.
    #[must_use]
    pub fn selection(&self) -> Option<&SelectionRange> {
        self.selection.as_ref()
    }

    /// Return the selected text as a string, reconstructed from the document's page text cache.
    ///
    /// Returns an empty string if nothing is selected or no document is available.
    #[must_use]
    pub fn selected_text(&self, ctx: &PluginContext<'_>) -> String {
        let Some(sel) = &self.selection else {
            return String::new();
        };
        let Some(doc) = ctx.document else {
            return String::new();
        };
        if sel.is_empty() {
            return String::new();
        }
        reconstruct_text(sel, doc)
    }

    /// Select all text in the document.
    pub fn select_all(&mut self, ctx: &PluginContext<'_>) {
        let Some(doc) = ctx.document else {
            return;
        };
        let page_count = doc.page_count();
        if page_count == 0 {
            return;
        }
        let last_page = PageIndex(page_count - 1);
        let Ok(last_text) = doc.page_text(last_page) else {
            return;
        };
        let last_char = last_text.glyphs.len();
        self.selection = Some(SelectionRange {
            start: CharIndex {
                page: PageIndex(0),
                char: 0,
            },
            end: CharIndex {
                page: last_page,
                char: last_char,
            },
        });
        self.selection_anchor = None;
    }

    /// Clear the current selection.
    pub fn deselect(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
    }

    /// Copy the selected text to the clipboard via egui's output command queue.
    pub fn copy_selected_text(&self, ctx: &PluginContext<'_>) {
        let text = self.selected_text(ctx);
        if !text.is_empty() {
            ctx.egui_ctx.copy_text(text);
        }
    }

    /// Hit-test: return the `CharIndex` of the glyph under `pos` on `page`, if any.
    ///
    /// Uses the cached `PageText` from `ctx.document`. Returns `None` if no
    /// document is loaded or no glyph's bbox contains `pos`.
    #[must_use]
    pub fn char_at_page_pos(
        &self,
        ctx: &PluginContext<'_>,
        page: PageIndex,
        pos: PointsPos,
    ) -> Option<CharIndex> {
        let doc = ctx.document?;
        let text = doc.page_text(page).ok()?;
        char_at_page_pos_with_text(&text, page, pos)
    }
}

fn reconstruct_text(sel: &SelectionRange, doc: &viewkai_engine::Document) -> String {
    let mut result = String::new();
    for page_idx in sel.start.page.0..=sel.end.page.0 {
        let page = PageIndex(page_idx);
        let Ok(text) = doc.page_text(page) else {
            continue;
        };
        let start_char = if page_idx == sel.start.page.0 {
            sel.start.char
        } else {
            0
        };
        let end_char = if page_idx == sel.end.page.0 {
            sel.end.char
        } else {
            text.glyphs.len()
        };
        for glyph in text.glyphs.get(start_char..end_char).unwrap_or(&[]) {
            result.push(glyph.char);
        }
        if page_idx < sel.end.page.0 {
            result.push('\n');
        }
    }
    result
}

pub(crate) fn apply_pointer_event(
    selection: &mut Option<SelectionRange>,
    anchor: &mut Option<CharIndex>,
    page: PageIndex,
    event: &crate::PointerEvent,
    ctx: &PluginContext<'_>,
) -> bool {
    let Some(doc) = ctx.document else {
        return false;
    };
    let Ok(text) = doc.page_text(page) else {
        return false;
    };

    if event.click_count >= 3 {
        if let Some(char_idx) = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt)
            && let Some(line) = text.lines.iter().find(|line| {
                line.page == page
                    && char_idx.char >= line.start_char
                    && char_idx.char < line.end_char
            })
        {
            *selection = Some(SelectionRange {
                start: CharIndex {
                    page,
                    char: line.start_char,
                },
                end: CharIndex {
                    page,
                    char: line.end_char,
                },
            });
            *anchor = None;
            return true;
        }
        return false;
    }

    if event.click_count == 2 {
        if let Some(char_idx) = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt)
            && let Some(word) = text.words.iter().find(|word| {
                word.page == page
                    && char_idx.char >= word.start_char
                    && char_idx.char < word.end_char
            })
        {
            *selection = Some(SelectionRange {
                start: CharIndex {
                    page,
                    char: word.start_char,
                },
                end: CharIndex {
                    page,
                    char: word.end_char,
                },
            });
            *anchor = None;
            return true;
        }
        return false;
    }

    if event.modifiers.shift && event.click_count == 1 && !event.primary_down {
        if let Some(target) = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt) {
            let existing_start = selection.as_ref().map(|current| current.start);
            if let Some(start) = existing_start {
                *selection = Some(SelectionRange::new(start, target));
            } else {
                *selection = Some(SelectionRange::new(target, target));
            }
            return true;
        }
        return false;
    }

    if event.primary_down
        && let Some(current_anchor) = *anchor
        && let Some(current) = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt)
    {
        *selection = Some(SelectionRange::new(current_anchor, current));
        return true;
    }

    if event.primary_down && event.click_count == 1 {
        if let Some(char_idx) = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt) {
            *anchor = Some(char_idx);
            *selection = Some(SelectionRange::new(char_idx, char_idx));
            return true;
        }
        *selection = None;
        *anchor = None;
        return false;
    }

    false
}

/// Pure hit-test helper (no `PluginContext` needed — testable with synthetic data).
pub(crate) fn char_at_page_pos_with_text(
    text: &viewkai_core::PageText,
    page: PageIndex,
    pos: PointsPos,
) -> Option<CharIndex> {
    text.glyphs.iter().enumerate().find_map(|(index, glyph)| {
        if point_in_rect(pos, glyph.bbox) {
            Some(CharIndex { page, char: index })
        } else {
            None
        }
    })
}

fn point_in_rect(pos: PointsPos, rect: PointsRect) -> bool {
    pos.x >= rect.x
        && pos.x <= rect.x + rect.width
        && pos.y >= rect.y
        && pos.y <= rect.y + rect.height
}

/// Convert a page-local `PointsRect` to screen-space `egui::Rect`.
fn page_rect_to_screen(bbox: PointsRect, page_origin: egui::Pos2, zoom: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(page_origin.x + bbox.x * zoom, page_origin.y + bbox.y * zoom),
        egui::vec2(bbox.width * zoom, bbox.height * zoom),
    )
}

impl Sealed for TextLayerPlugin {}

impl ViewerPlugin for TextLayerPlugin {
    fn id(&self) -> &'static str {
        "viewkai.text_layer"
    }

    fn draw_page_overlay(&mut self, page: PageIndex, ui: &mut Ui, ctx: &mut PluginContext<'_>) {
        let Some(doc) = ctx.document else {
            return;
        };
        let Ok(text) = doc.page_text(page) else {
            return;
        };

        let page_origin = ui.min_rect().min;
        let zoom = ctx.zoom;

        if self.debug {
            for word in &text.words {
                let screen_rect = page_rect_to_screen(word.bbox, page_origin, zoom);
                ui.painter().rect_stroke(
                    screen_rect,
                    0.0,
                    Stroke::new(1.0, Color32::RED),
                    egui::StrokeKind::Outside,
                );
            }
        }

        if let Some(sel) = &self.selection {
            let start_char = if sel.start.page == page {
                sel.start.char
            } else if sel.start.page.0 < page.0 {
                0
            } else {
                return;
            };
            let end_char = if sel.end.page == page {
                sel.end.char
            } else if sel.end.page.0 > page.0 {
                text.glyphs.len()
            } else {
                return;
            };

            if start_char >= end_char {
                return;
            }

            for line in &text.lines {
                if line.page != page {
                    continue;
                }
                let line_start = line.start_char.max(start_char);
                let line_end = line.end_char.min(end_char);
                if line_start >= line_end {
                    continue;
                }

                let selected_glyphs = text.glyphs.get(line_start..line_end).unwrap_or(&[]);
                if selected_glyphs.is_empty() {
                    continue;
                }

                let mut union_rect = selected_glyphs[0].bbox;
                for glyph in selected_glyphs.iter().skip(1) {
                    let right =
                        (union_rect.x + union_rect.width).max(glyph.bbox.x + glyph.bbox.width);
                    let bottom = (union_rect.y + union_rect.height)
                        .max(glyph.bbox.y + glyph.bbox.height);
                    union_rect.x = union_rect.x.min(glyph.bbox.x);
                    union_rect.y = union_rect.y.min(glyph.bbox.y);
                    union_rect.width = right - union_rect.x;
                    union_rect.height = bottom - union_rect.y;
                }

                let screen_rect = page_rect_to_screen(union_rect, page_origin, zoom);
                ui.painter()
                    .rect_filled(screen_rect, 0.0, ctx.selection_color);
            }
        }
    }

    fn show_toolbar(&mut self, ui: &mut Ui, _ctx: &mut PluginContext<'_>) {
        ui.checkbox(&mut self.debug, "Show text layer");
    }

    fn on_frame_update(&mut self, ctx: &mut PluginContext<'_>) {
        if !ctx.library_shortcuts_enabled {
            return;
        }

        let ctrl_a = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::A);
        let ctrl_c = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::C);
        let esc = egui::Key::Escape;

        if ctx.egui_ctx.input_mut(|input| input.consume_shortcut(&ctrl_a)) {
            self.select_all(ctx);
        } else if ctx.egui_ctx.input_mut(|input| input.consume_shortcut(&ctrl_c)) {
            self.copy_selected_text(ctx);
        } else if ctx.egui_ctx.input(|input| input.key_pressed(esc)) {
            self.deselect();
        }
    }

    fn on_pointer_event(
        &mut self,
        page: PageIndex,
        event: &crate::PointerEvent,
        ctx: &mut PluginContext<'_>,
    ) -> bool {
        apply_pointer_event(
            &mut self.selection,
            &mut self.selection_anchor,
            page,
            event,
            ctx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use viewkai_core::{GlyphBox, PageText, WordSpan};

    fn make_glyph(ch: char, x: f32, y: f32, w: f32, h: f32) -> GlyphBox {
        GlyphBox {
            char: ch,
            bbox: PointsRect {
                x,
                y,
                width: w,
                height: h,
            },
            font_size_pt: 12.0,
            rotation_deg: 0.0,
        }
    }

    #[test]
    fn debug_toggle_roundtrips() {
        let mut plugin = TextLayerPlugin::new();
        assert!(!plugin.debug());
        plugin.set_debug(true);
        assert!(plugin.debug());
        plugin.set_debug(false);
        assert!(!plugin.debug());
    }

    #[test]
    fn char_at_pos_hits_glyph_pure() {
        let page = PageIndex(0);
        let glyphs = vec![make_glyph('A', 10.0, 20.0, 8.0, 12.0)];
        let text = PageText {
            glyphs,
            words: vec![],
            lines: vec![],
        };
        let pos = PointsPos { x: 14.0, y: 26.0 };
        let result = char_at_page_pos_with_text(&text, page, pos);
        assert_eq!(result, Some(CharIndex { page, char: 0 }));
    }

    #[test]
    fn char_at_pos_misses_returns_none() {
        let page = PageIndex(0);
        let glyphs = vec![make_glyph('A', 10.0, 20.0, 8.0, 12.0)];
        let text = PageText {
            glyphs,
            words: vec![],
            lines: vec![],
        };
        let pos = PointsPos { x: 100.0, y: 100.0 };
        let result = char_at_page_pos_with_text(&text, page, pos);
        assert_eq!(result, None);
    }

    #[test]
    fn selection_normalize_forward() {
        let page = PageIndex(0);
        let a = CharIndex {
            page,
            char: 10,
        };
        let b = CharIndex { page, char: 5 };
        let sel = SelectionRange::new(a, b);
        assert_eq!(sel.start.char, 5);
        assert_eq!(sel.end.char, 10);
    }

    #[test]
    fn selection_normalize_cross_page() {
        let a = CharIndex {
            page: PageIndex(5),
            char: 0,
        };
        let b = CharIndex {
            page: PageIndex(3),
            char: 0,
        };
        let sel = SelectionRange::new(a, b);
        assert_eq!(sel.start.page, PageIndex(3));
        assert_eq!(sel.end.page, PageIndex(5));
    }

    #[test]
    fn double_click_selects_word_pure() {
        let page = PageIndex(0);
        let glyphs = vec![
            make_glyph('h', 0.0, 0.0, 5.0, 10.0),
            make_glyph('i', 5.0, 0.0, 5.0, 10.0),
        ];
        let words = vec![WordSpan {
            page,
            start_char: 0,
            end_char: 2,
            bbox: PointsRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
        }];
        let text = PageText {
            glyphs,
            words,
            lines: vec![],
        };
        let event = crate::PointerEvent {
            pos_in_page_pt: PointsPos { x: 3.0, y: 5.0 },
            primary_down: false,
            modifiers: egui::Modifiers::NONE,
            click_count: 2,
        };
        let char_idx = char_at_page_pos_with_text(&text, page, event.pos_in_page_pt);
        assert_eq!(char_idx, Some(CharIndex { page, char: 0 }));
        let word = text.words.iter().find(|word| {
            word.page == page
                && char_idx.expect("char exists").char >= word.start_char
                && char_idx.expect("char exists").char < word.end_char
        });
        assert!(word.is_some());
        assert_eq!(word.expect("word exists").start_char, 0);
        assert_eq!(word.expect("word exists").end_char, 2);
    }
}
