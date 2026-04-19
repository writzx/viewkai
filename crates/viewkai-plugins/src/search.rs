//! Search plugin: full-text search with Ctrl+F overlay and match highlighting.

use egui::{Area, Color32, Context, Id, Key, KeyboardShortcut, Modifiers, Order, Ui};
use viewkai_core::{PageIndex, SearchMatch, SearchQuery, SearchState};

use crate::{PluginContext, ViewerPlugin, sealed::Sealed};

/// Plugin that provides full-text search with per-page match highlighting and
/// a Ctrl+F viewer-level overlay.
#[derive(Debug)]
pub struct SearchPlugin {
    state: Option<SearchState>,
    open: bool,
    focus_input_on_next_frame: bool,
    match_color: Color32,
    current_match_color: Color32,
    chunk_size: usize,
    query_input: String,
}

impl Default for SearchPlugin {
    fn default() -> Self {
        Self {
            state: None,
            open: false,
            focus_input_on_next_frame: false,
            match_color: Color32::from_rgba_unmultiplied(255, 255, 0, 120),
            current_match_color: Color32::from_rgba_unmultiplied(255, 150, 0, 180),
            chunk_size: 10,
            query_input: String::new(),
        }
    }
}

impl SearchPlugin {
    /// Create a new search plugin instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether the search overlay is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open the search overlay and focus the input.
    pub fn open(&mut self) {
        self.open = true;
        self.focus_input_on_next_frame = true;
    }

    /// Close the search overlay and clear state.
    pub fn close(&mut self) {
        self.open = false;
        self.state = None;
        self.focus_input_on_next_frame = false;
    }

    /// Return the current search state, if any.
    #[must_use]
    pub fn state(&self) -> Option<&SearchState> {
        self.state.as_ref()
    }

    /// Update the search query and restart the search.
    pub fn update_query(&mut self, query: SearchQuery, ctx: &PluginContext<'_>) {
        if query.term.is_empty() {
            self.state = Some(SearchState {
                query,
                matches: Vec::new(),
                current_match: 0,
                pending_pages: Vec::new(),
            });
            return;
        }

        let page_count = ctx.document.map_or(0, viewkai_engine::Document::page_count);
        self.state = Some(SearchState {
            query,
            matches: Vec::new(),
            current_match: 0,
            pending_pages: (0..page_count).map(PageIndex).collect(),
        });
    }

    /// Advance to the next match and return it.
    pub fn next_match(&mut self) -> Option<&SearchMatch> {
        let state = self.state.as_mut()?;
        if state.matches.is_empty() {
            return None;
        }
        state.current_match = (state.current_match + 1) % state.matches.len();
        state.matches.get(state.current_match)
    }

    /// Go to the previous match and return it.
    pub fn prev_match(&mut self) -> Option<&SearchMatch> {
        let state = self.state.as_mut()?;
        if state.matches.is_empty() {
            return None;
        }
        if state.current_match == 0 {
            state.current_match = state.matches.len() - 1;
        } else {
            state.current_match -= 1;
        }
        state.matches.get(state.current_match)
    }

    /// Return the current match.
    #[must_use]
    pub fn current_match(&self) -> Option<&SearchMatch> {
        let state = self.state.as_ref()?;
        state.matches.get(state.current_match)
    }

    /// Set the color for non-current match highlights.
    pub fn set_match_color(&mut self, color: Color32) {
        self.match_color = color;
    }

    /// Set the color for the current match highlight.
    pub fn set_current_match_color(&mut self, color: Color32) {
        self.current_match_color = color;
    }

    fn process_pending_chunk(&mut self, ctx: &PluginContext<'_>) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let Some(doc) = ctx.document else {
            return;
        };

        let pages_to_process = state
            .pending_pages
            .drain(..self.chunk_size.min(state.pending_pages.len()))
            .collect::<Vec<_>>();

        for page in pages_to_process {
            if let Ok(page_matches) = viewkai_engine::search_page(doc, page, &state.query) {
                state.matches.extend(page_matches);
            }
        }

        state
            .matches
            .sort_by_key(|search_match| (search_match.page.0, search_match.char_span.start));
        if state.matches.is_empty() {
            state.current_match = 0;
        } else {
            state.current_match = state.current_match.min(state.matches.len() - 1);
        }
    }

    fn navigation_label(state: &SearchState) -> String {
        let current = if state.matches.is_empty() {
            0
        } else {
            state.current_match + 1
        };
        let suffix = if state.pending_pages.is_empty() { "" } else { "+" };
        format!("{current} of {}{suffix}", state.matches.len())
    }

    fn request_scroll_to_match(search_match: &SearchMatch, ctx: &mut PluginContext<'_>) {
        if let Some(rect) = search_match.rects.first().copied() {
            ctx.request_scroll_to(search_match.page, rect);
            ctx.request_repaint();
        }
    }
}

impl Sealed for SearchPlugin {}

impl ViewerPlugin for SearchPlugin {
    fn id(&self) -> &'static str {
        "viewkai.search"
    }

    fn on_frame_update(&mut self, ctx: &mut PluginContext<'_>) {
        if ctx.library_shortcuts_enabled {
            let command_f = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
            if ctx
                .egui_ctx
                .input_mut(|input| input.consume_shortcut(&command_f))
            {
                if self.open {
                    self.close();
                } else {
                    self.open();
                }
            }
        }

        if self.open
            && ctx.library_shortcuts_enabled
            && ctx.egui_ctx.input(|input| input.key_pressed(Key::Escape))
        {
            self.close();
        }

        self.process_pending_chunk(ctx);

        if self
            .state
            .as_ref()
            .is_some_and(|state| !state.pending_pages.is_empty())
        {
            ctx.request_repaint();
        }
    }

    fn draw_page_overlay(&mut self, page: PageIndex, ui: &mut Ui, ctx: &mut PluginContext<'_>) {
        let Some(state) = &self.state else {
            return;
        };
        if state.matches.is_empty() {
            return;
        }

        let page_origin = ui.min_rect().min;
        let zoom = ctx.zoom;

        for (index, search_match) in state.matches.iter().enumerate() {
            if search_match.page != page {
                continue;
            }

            let color = if index == state.current_match {
                self.current_match_color
            } else {
                self.match_color
            };

            for rect in &search_match.rects {
                let screen_rect = egui::Rect::from_min_size(
                    egui::pos2(page_origin.x + rect.x * zoom, page_origin.y + rect.y * zoom),
                    egui::vec2(rect.width * zoom, rect.height * zoom),
                );
                ui.painter().rect_filled(screen_rect, 0.0, color);
            }
        }
    }

    fn show_toolbar(&mut self, ui: &mut Ui, _ctx: &mut PluginContext<'_>) {
        if ui.button("Find").clicked() {
            self.open();
        }

        if let Some(state) = &self.state {
            ui.label(Self::navigation_label(state));
        }
    }

    fn show_viewer_overlay(&mut self, egui_ctx: &Context, ctx: &mut PluginContext<'_>) {
        if !self.open {
            return;
        }

        let mut close = false;
        let mut go_next = false;
        let mut go_prev = false;
        let mut query_changed = false;
        let mut case_sensitive = self
            .state
            .as_ref()
            .is_some_and(|state| state.query.case_sensitive);
        let mut whole_word = self
            .state
            .as_ref()
            .is_some_and(|state| state.query.whole_word);

        Area::new(Id::new("viewkai_search"))
            .order(Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
            .show(egui_ctx, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.query_input)
                                .hint_text("Search…")
                                .desired_width(200.0),
                        );

                        if self.focus_input_on_next_frame {
                            response.request_focus();
                            self.focus_input_on_next_frame = false;
                        }

                        if response.changed() {
                            query_changed = true;
                        }

                        if response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                            if ui.input(|input| input.modifiers.shift) {
                                go_prev = true;
                            } else {
                                go_next = true;
                            }
                        }

                        if ui.button("✕").clicked() {
                            close = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut case_sensitive, "Aa").changed() {
                            query_changed = true;
                        }
                        if ui.checkbox(&mut whole_word, "W").changed() {
                            query_changed = true;
                        }

                        if let Some(state) = &self.state {
                            ui.label(Self::navigation_label(state));
                        }
                    });
                });
            });

        if close {
            self.close();
            return;
        }

        if query_changed {
            self.update_query(
                SearchQuery {
                    term: self.query_input.clone(),
                    case_sensitive,
                    whole_word,
                },
                ctx,
            );
        }

        if go_next {
            if let Some(search_match) = self.next_match().cloned() {
                Self::request_scroll_to_match(&search_match, ctx);
            }
        } else if go_prev && let Some(search_match) = self.prev_match().cloned() {
            Self::request_scroll_to_match(&search_match, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use viewkai_core::text::CharSpan;

    use super::*;

    #[test]
    fn open_close_toggles_state() {
        let mut plugin = SearchPlugin::new();
        assert!(!plugin.is_open());
        plugin.open();
        assert!(plugin.is_open());
        plugin.close();
        assert!(!plugin.is_open());
        assert!(plugin.state().is_none());
    }

    #[test]
    fn next_match_wraps_around() {
        let mut plugin = SearchPlugin::new();
        plugin.state = Some(SearchState {
            query: SearchQuery::default(),
            matches: vec![
                SearchMatch {
                    page: PageIndex(0),
                    char_span: CharSpan {
                        page: PageIndex(0),
                        start: 0,
                        end: 1,
                    },
                    rects: vec![],
                },
                SearchMatch {
                    page: PageIndex(0),
                    char_span: CharSpan {
                        page: PageIndex(0),
                        start: 1,
                        end: 2,
                    },
                    rects: vec![],
                },
            ],
            current_match: 1,
            pending_pages: vec![],
        });
        plugin.next_match();
        assert_eq!(plugin.state().expect("state").current_match, 0);
    }

    #[test]
    fn prev_match_wraps_around() {
        let mut plugin = SearchPlugin::new();
        plugin.state = Some(SearchState {
            query: SearchQuery::default(),
            matches: vec![
                SearchMatch {
                    page: PageIndex(0),
                    char_span: CharSpan {
                        page: PageIndex(0),
                        start: 0,
                        end: 1,
                    },
                    rects: vec![],
                },
                SearchMatch {
                    page: PageIndex(0),
                    char_span: CharSpan {
                        page: PageIndex(0),
                        start: 1,
                        end: 2,
                    },
                    rects: vec![],
                },
            ],
            current_match: 0,
            pending_pages: vec![],
        });
        plugin.prev_match();
        assert_eq!(plugin.state().expect("state").current_match, 1);
    }
}
