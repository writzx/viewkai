//! Outline plugin: bookmark tree sidebar with destination navigation.

use std::{collections::HashSet, sync::Arc};

use egui::{CollapsingHeader, RichText, ScrollArea, Ui};
use viewkai_core::{Destination, DestPosition, Outline, OutlineNodeId, PageIndex, PointsRect};
use viewkai_engine::Document;

use crate::{PluginContext, ViewerPlugin, sealed};

/// Plugin that renders a document outline sidebar.
#[derive(Debug)]
pub struct OutlinePlugin {
    visible: bool,
    expanded_nodes: HashSet<OutlineNodeId>,
    last_target_page: Option<PageIndex>,
    outline_cache: Option<Arc<Outline>>,
    pending_destination: Option<Destination>,
}

impl OutlinePlugin {
    /// Create a new outline plugin instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the outline sidebar panel.
    pub fn render_panel(&mut self, ui: &mut Ui, document: Option<&Document>) {
        if let Some(doc) = document
            && let Ok(outline) = doc.outline()
        {
            self.outline_cache = Some(outline);
        }

        let Some(outline) = self.outline_cache.clone() else {
            ui.label("No document loaded");
            return;
        };

        if outline.is_empty() {
            ui.label("No outline");
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            for &root in &outline.roots {
                self.render_node(ui, &outline, root);
            }
        });
    }

    /// Queue a destination jump to be applied during the next frame update.
    pub fn set_pending_destination(&mut self, dest: Destination) {
        self.pending_destination = Some(dest);
    }

    /// Return whether the outline panel is visible.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Show or hide the outline panel.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Return the currently queued destination, if any.
    #[must_use]
    pub fn pending_destination(&self) -> Option<&Destination> {
        self.pending_destination.as_ref()
    }

    fn render_node(&mut self, ui: &mut Ui, outline: &Outline, node_id: OutlineNodeId) {
        let Some(node) = outline.node(node_id) else {
            return;
        };

        let is_current = node
            .destination
            .as_ref()
            .zip(self.last_target_page)
            .is_some_and(|(dest, current)| dest.page == current);
        let label = if is_current {
            RichText::new(&node.title).strong()
        } else {
            RichText::new(&node.title)
        };

        if node.children.is_empty() {
            if ui.selectable_label(is_current, label).clicked()
                && let Some(dest) = node.destination.clone()
            {
                self.pending_destination = Some(dest);
            }
            return;
        }

        let was_open = self.expanded_nodes.contains(&node_id);
        let response = CollapsingHeader::new(label)
            .default_open(was_open)
            .show(ui, |ui| {
                for &child in &node.children {
                    self.render_node(ui, outline, child);
                }
            });

        if response.header_response.clicked() {
            if was_open {
                self.expanded_nodes.remove(&node_id);
            } else {
                self.expanded_nodes.insert(node_id);
            }

            if let Some(dest) = node.destination.clone() {
                self.pending_destination = Some(dest);
            }
        }
    }

    fn destination_rect(dest: &Destination) -> PointsRect {
        match &dest.position {
            Some(DestPosition::Point { x_pt, y_pt }) => PointsRect {
                x: *x_pt,
                y: *y_pt,
                width: 1.0,
                height: 1.0,
            },
            Some(DestPosition::FitRect { bbox }) => *bbox,
            Some(DestPosition::FitWidth { y_pt }) => PointsRect {
                x: 0.0,
                y: y_pt.unwrap_or(0.0),
                width: 1.0,
                height: 1.0,
            },
            Some(DestPosition::FitHeight { x_pt }) => PointsRect {
                x: x_pt.unwrap_or(0.0),
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            Some(DestPosition::FitPage) | None => PointsRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        }
    }
}

impl Default for OutlinePlugin {
    fn default() -> Self {
        Self {
            visible: false,
            expanded_nodes: HashSet::new(),
            last_target_page: None,
            outline_cache: None,
            pending_destination: None,
        }
    }
}

impl sealed::Sealed for OutlinePlugin {}

impl ViewerPlugin for OutlinePlugin {
    fn id(&self) -> &'static str {
        "viewkai.outline"
    }

    fn show_toolbar(&mut self, ui: &mut Ui, _ctx: &mut PluginContext<'_>) {
        ui.checkbox(&mut self.visible, "Show Outline");
    }

    fn on_frame_update(&mut self, ctx: &mut PluginContext<'_>) {
        if let Some(doc) = ctx.document
            && let Ok(outline) = doc.outline()
        {
            self.outline_cache = Some(outline);
        }

        if let Some(&first) = ctx.visible_pages.first() {
            self.last_target_page = Some(first);
        }

        if let Some(dest) = self.pending_destination.take() {
            ctx.request_scroll_to(dest.page, Self::destination_rect(&dest));
            ctx.request_repaint();
        }
    }
}
