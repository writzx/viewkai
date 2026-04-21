//! Native PDF viewer application built on `viewkai`.

use eframe::egui;
use std::sync::{Arc, Mutex};
use viewkai::{RotationDelta, ViewMode, Viewer, zoom::ZoomState};
use viewkai_core::PageIndex;
use viewkai_engine::{Document, init};

mod zoom_ui;

type PendingLoadSink = Arc<Mutex<Option<Result<PendingLoad, String>>>>;

struct PendingLoad {
    bytes: Vec<u8>,
    source_label: String,
    document_name: Option<String>,
}

#[derive(Default)]
struct UrlDialog {
    visible: bool,
    url_buffer: String,
}

/// Current loading lifecycle for the native application.
pub enum LoadState {
    /// No document is loaded and no load is in progress.
    Idle,
    /// The app is currently fetching or opening document bytes.
    AcquiringBytes {
        /// Status label shown while bytes are being acquired.
        label: String,
    },
    /// A document loaded successfully.
    Loaded,
    /// Loading failed with a user-displayable message.
    Failed {
        /// User-facing error message describing the load failure.
        msg: String,
    },
}

/// Native application embedding the `viewkai` viewer widget.
pub struct App {
    viewer: Viewer,
    load_state: LoadState,
    debug_info: Option<String>,
    page_input: String,
    page_input_focused: bool,
    total_pages: usize,
    sidebar_tab: SidebarTab,
    current_document_name: Option<String>,
    last_viewport_title: String,
    show_about: bool,
    url_dialog: UrlDialog,
    pending_load: Option<PendingLoadSink>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Outline,
    Thumbnails,
}

enum LoadEvent {
    BytesReceived(PendingLoad),
    LoadSucceeded,
    LoadFailed { message: String },
    Reset,
}

enum ShortcutAction {
    ResetZoom,
    FitWidth,
    FitPage,
    ZoomIn,
    ZoomOut,
    OpenFile,
    OpenUrl,
    CloseDocument,
    Exit,
}

const SHORTCUTS: &[(egui::Modifiers, egui::Key, ShortcutAction)] = &[
    (
        egui::Modifiers::CTRL,
        egui::Key::Num0,
        ShortcutAction::ResetZoom,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::Num1,
        ShortcutAction::FitWidth,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::Num2,
        ShortcutAction::FitPage,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::Plus,
        ShortcutAction::ZoomIn,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::Equals,
        ShortcutAction::ZoomIn,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::Minus,
        ShortcutAction::ZoomOut,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::O,
        ShortcutAction::OpenFile,
    ),
    (egui::Modifiers::CTRL, egui::Key::L, ShortcutAction::OpenUrl),
    (
        egui::Modifiers::CTRL,
        egui::Key::W,
        ShortcutAction::CloseDocument,
    ),
    (egui::Modifiers::CTRL, egui::Key::Q, ShortcutAction::Exit),
];

const SHORTCUT_FIND_PREV_ALT: egui::KeyboardShortcut = egui::KeyboardShortcut::new(
    egui::Modifiers {
        command: true,
        shift: true,
        ..egui::Modifiers::NONE
    },
    egui::Key::G,
);
const SHORTCUT_FIND_NEXT_ALT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::G);
const SHORTCUT_FIND_PREV: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::F3);
const SHORTCUT_FIND_NEXT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F3);
const SHORTCUT_OUTLINE_TOGGLE: egui::KeyboardShortcut = egui::KeyboardShortcut::new(
    egui::Modifiers {
        ctrl: true,
        shift: true,
        ..egui::Modifiers::NONE
    },
    egui::Key::O,
);
const SHORTCUT_THUMBNAILS_TOGGLE: egui::KeyboardShortcut = egui::KeyboardShortcut::new(
    egui::Modifiers {
        ctrl: true,
        shift: true,
        ..egui::Modifiers::NONE
    },
    egui::Key::T,
);
const VIEW_MODE_OPTIONS: [(&str, ViewMode); 4] = [
    ("Single Page", ViewMode::Single),
    ("Continuous", ViewMode::Continuous),
    (
        "Spread (Cover Alone)",
        ViewMode::Spread {
            cover_separate: true,
        },
    ),
    (
        "Spread (All Pairs)",
        ViewMode::Spread {
            cover_separate: false,
        },
    ),
];

impl App {
    /// Create a new native app instance.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = cc;

        Self {
            viewer: Viewer::new(),
            load_state: LoadState::Idle,
            debug_info: None,
            page_input: String::new(),
            page_input_focused: false,
            total_pages: 0,
            sidebar_tab: SidebarTab::Outline,
            current_document_name: None,
            last_viewport_title: String::new(),
            show_about: false,
            url_dialog: UrlDialog::default(),
            pending_load: None,
        }
    }

    fn transition(&mut self, ev: LoadEvent) {
        match ev {
            LoadEvent::BytesReceived(PendingLoad {
                bytes,
                source_label,
                document_name,
            }) => {
                self.load_state = LoadState::AcquiringBytes {
                    label: source_label,
                };
                self.load_bytes(&bytes, document_name);
            }
            LoadEvent::LoadSucceeded => {
                self.load_state = LoadState::Loaded;
            }
            LoadEvent::LoadFailed { message } => {
                self.current_document_name = None;
                self.load_state = LoadState::Failed { msg: message };
            }
            LoadEvent::Reset => {
                self.viewer.clear();
                self.load_state = LoadState::Idle;
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
                self.current_document_name = None;
            }
        }
    }

    /// Load a PDF from bytes without going through the file dialog.
    /// Transitions `LoadState` to `Loaded` on success.
    ///
    /// # Errors
    ///
    /// Returns the user-facing load failure message when parsing or rendering the
    /// provided PDF bytes fails.
    // justify: the app test/helpers hand owned fixture buffers through this API
    // and retaining `Vec<u8>` avoids a public signature churn for little benefit.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_bytes_sync(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.load_bytes(&bytes, None);
        match &self.load_state {
            LoadState::Loaded => Ok(()),
            LoadState::Failed { msg } => Err(msg.clone()),
            _ => Err("unexpected state after load".to_owned()),
        }
    }

    /// Returns a reference to the inner viewer for inspection.
    #[must_use]
    pub fn viewer(&self) -> &Viewer {
        &self.viewer
    }

    /// Returns the current application loading state.
    #[must_use]
    pub fn load_state(&self) -> &LoadState {
        &self.load_state
    }

    /// Returns whether the URL dialog is visible.
    #[must_use]
    pub fn url_dialog_visible(&self) -> bool {
        self.url_dialog.visible
    }

    /// Returns whether the About dialog is visible.
    #[must_use]
    pub fn about_visible(&self) -> bool {
        self.show_about
    }

    fn describe_pdf(bytes: &[u8]) -> Result<String, String> {
        let doc = Document::from_bytes(bytes.to_vec()).map_err(|err| err.to_string())?;
        let size = doc.page_size(PageIndex(0)).map_or_else(
            |_| "unknown".to_owned(),
            |page| format!("{:.1}x{:.1}", page.width_pt, page.height_pt),
        );

        Ok(format!(
            "PDF loaded: {} pages. Page 1 size: {} points.",
            doc.page_count(),
            size
        ))
    }

    fn load_bytes(&mut self, bytes: &[u8], document_name: Option<String>) {
        match self.viewer.load_bytes(bytes.to_owned()) {
            Ok(()) => {
                self.total_pages = self.viewer.page_count();
                self.page_input = if self.total_pages > 0 {
                    "1".to_owned()
                } else {
                    String::new()
                };
                self.debug_info =
                    Some(Self::describe_pdf(bytes).unwrap_or_else(|err| {
                        format!("PDF loaded; debug info unavailable: {err}")
                    }));
                self.current_document_name = document_name;
                self.transition(LoadEvent::LoadSucceeded);
            }
            Err(err) => {
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
                self.current_document_name = None;
                self.transition(LoadEvent::LoadFailed {
                    message: err.to_string(),
                });
            }
        }
    }

    fn open_file(&mut self) {
        self.load_state = LoadState::AcquiringBytes {
            label: "Opening file…".to_owned(),
        };

        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        else {
            self.load_state = LoadState::Idle;
            return;
        };

        match std::fs::read(&path) {
            Ok(bytes) => self.transition(LoadEvent::BytesReceived(PendingLoad {
                document_name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                bytes,
                source_label: format!("Reading {}", path.display()),
            })),
            Err(err) => {
                self.debug_info = None;
                self.viewer.clear();
                self.current_document_name = None;
                self.transition(LoadEvent::LoadFailed {
                    message: format!("Failed to read {}: {err}", path.display()),
                });
            }
        }
    }

    fn dismiss_error(&mut self) {
        self.transition(LoadEvent::Reset);
    }

    fn viewport_title(&self) -> String {
        self.current_document_name
            .as_deref()
            .map_or_else(|| "viewkai".to_owned(), |name| format!("{name} — viewkai"))
    }

    fn sync_viewport_title(&mut self, ctx: &egui::Context) {
        let next_title = self.viewport_title();
        if self.last_viewport_title != next_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(next_title.clone()));
            self.last_viewport_title = next_title;
        }
    }

    fn toggle_outline_visible(&mut self, visible: bool) {
        self.viewer.outline_mut().set_visible(visible);
        if visible {
            self.sidebar_tab = SidebarTab::Outline;
        }
    }

    fn toggle_thumbnails_visible(&mut self, visible: bool) {
        self.viewer.thumbnails_mut().set_visible(visible);
        if visible {
            self.sidebar_tab = SidebarTab::Thumbnails;
        }
    }

    fn document_name_from_url(url: &str) -> Option<String> {
        let trimmed = url.trim();
        let without_fragment = trimmed.split('#').next().unwrap_or(trimmed);
        let without_query = without_fragment
            .split('?')
            .next()
            .unwrap_or(without_fragment);
        let name = without_query
            .rsplit('/')
            .next()
            .unwrap_or(without_query)
            .trim();
        (!name.is_empty()).then(|| name.to_owned())
    }

    fn show_url_dialog(&mut self) {
        self.url_dialog.visible = true;
    }

    fn close_document(&mut self) {
        self.transition(LoadEvent::Reset);
    }

    fn begin_url_fetch(&mut self, ctx: &egui::Context, url: String) {
        let sink = Arc::new(Mutex::new(None));
        fetch_url_native(
            &url,
            Self::document_name_from_url(&url),
            ctx,
            Arc::clone(&sink),
        );
        self.pending_load = Some(sink);
        self.load_state = LoadState::AcquiringBytes {
            label: format!("Fetching {url}"),
        };
    }

    fn poll_pending_load(&mut self) {
        if let Some(pending) = self.pending_load.as_ref().map(Arc::clone)
            && let Ok(mut guard) = pending.try_lock()
            && let Some(result) = guard.take()
        {
            self.pending_load = None;
            match result {
                Ok(pending) => self.transition(LoadEvent::BytesReceived(pending)),
                Err(message) => self.transition(LoadEvent::LoadFailed { message }),
            }
        }
    }

    fn show_loading(ui: &mut egui::Ui, label: &str) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new());
                ui.add_space(8.0);
                ui.label(label);
            });
        });
    }

    fn show_failure(&mut self, ui: &mut egui::Ui, msg: &str) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                ui.add_space(8.0);
                if ui.button("Dismiss").clicked() {
                    self.dismiss_error();
                }
            });
        });
    }

    fn apply_zoom_factor(&mut self, factor: f32) {
        let current = match self.viewer.zoom() {
            ZoomState::Discrete(z) | ZoomState::Custom(z) => z,
            ZoomState::FitWidth | ZoomState::FitPage => 1.0,
        };
        self.viewer
            .set_zoom(ZoomState::Custom((current * factor).clamp(0.1, 8.0)));
    }

    fn jump_to_page_input(&mut self) {
        if let Ok(page_num) = self.page_input.trim().parse::<usize>()
            && page_num >= 1
            && page_num <= self.total_pages
        {
            self.viewer.scroll_to_page(page_num - 1);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // IMPORTANT: Most-specific shortcuts (more modifiers) MUST be consumed first.
        // egui uses inclusive modifier matching: Ctrl+O matches when Ctrl+Shift+O is pressed.
        // Ctrl+Shift+* shortcuts must be handled before Ctrl+* shortcuts to avoid conflicts.

        // --- Ctrl+Shift shortcuts (most specific — handle first) ---
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_OUTLINE_TOGGLE)) {
            let is_visible = self.viewer.outline().visible();
            self.toggle_outline_visible(!is_visible);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_THUMBNAILS_TOGGLE)) {
            let is_visible = self.viewer.thumbnails().visible();
            self.toggle_thumbnails_visible(!is_visible);
        }

        // --- Cmd+Shift / Shift shortcuts ---
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV_ALT)) {
            self.viewer.prev_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV)) {
            self.viewer.prev_match();
        }

        // --- Cmd / single-modifier shortcuts (least specific — handle last) ---
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT_ALT)) {
            self.viewer.next_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT)) {
            self.viewer.next_match();
        }

        for (mods, key, action) in SHORTCUTS {
            let shortcut = egui::KeyboardShortcut::new(*mods, *key);
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                self.apply_shortcut_action(ctx, action);
            }
        }
    }

    fn active_sidebar_tab(&self) -> Option<SidebarTab> {
        let outline_visible = self.viewer.outline().visible();
        let thumbnails_visible = self.viewer.thumbnails().visible();

        match (outline_visible, thumbnails_visible) {
            (false, false) => None,
            (true, false) => Some(SidebarTab::Outline),
            (false, true) => Some(SidebarTab::Thumbnails),
            (true, true) => Some(self.sidebar_tab),
        }
    }

    fn apply_shortcut_action(&mut self, ctx: &egui::Context, action: &ShortcutAction) {
        match action {
            ShortcutAction::ResetZoom => self.viewer.set_zoom(ZoomState::Discrete(1.0)),
            ShortcutAction::FitWidth => self.viewer.set_zoom(ZoomState::FitWidth),
            ShortcutAction::FitPage => self.viewer.set_zoom(ZoomState::FitPage),
            ShortcutAction::ZoomIn => {
                let z = zoom_ui::step_zoom_up(self.viewer.zoom());
                self.viewer.set_zoom(z);
            }
            ShortcutAction::ZoomOut => {
                let z = zoom_ui::step_zoom_down(self.viewer.zoom());
                self.viewer.set_zoom(z);
            }
            ShortcutAction::OpenFile => self.open_file(),
            ShortcutAction::OpenUrl => self.show_url_dialog(),
            ShortcutAction::CloseDocument => self.close_document(),
            ShortcutAction::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    fn view_mode_selector_ui(&mut self, ui: &mut egui::Ui) {
        let mut selected = self.viewer.view_mode();
        for (label, mode) in VIEW_MODE_OPTIONS {
            ui.radio_value(&mut selected, mode, label);
        }
        if selected != self.viewer.view_mode() {
            self.viewer.set_view_mode(selected);
        }
    }

    fn view_mode_menu_ui(&mut self, ui: &mut egui::Ui) {
        let mut selected = self.viewer.view_mode();
        for (label, mode) in VIEW_MODE_OPTIONS {
            if ui.radio_value(&mut selected, mode, label).clicked() {
                self.viewer.set_view_mode(selected);
            }
        }
    }

    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open File… (Ctrl+O)").clicked() {
                    ui.close();
                    self.open_file();
                }
                if ui.button("Open from URL… (Ctrl+L)").clicked() {
                    ui.close();
                    self.show_url_dialog();
                }
                if ui.button("Close (Ctrl+W)").clicked() {
                    ui.close();
                    self.close_document();
                }
                if ui.button("Exit (Ctrl+Q)").clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("View", |ui| {
                ui.menu_button("View Mode", |ui| {
                    self.view_mode_menu_ui(ui);
                });
                ui.menu_button("Sidebar", |ui| {
                    let mut show_outline = self.viewer.outline().visible();
                    if ui.checkbox(&mut show_outline, "Show Outline").clicked() {
                        self.toggle_outline_visible(show_outline);
                    }

                    let mut show_thumbnails = self.viewer.thumbnails().visible();
                    if ui
                        .checkbox(&mut show_thumbnails, "Show Thumbnails")
                        .clicked()
                    {
                        self.toggle_thumbnails_visible(show_thumbnails);
                    }
                });
                ui.menu_button("Rotation", |ui| {
                    if ui.button("Rotate Left (Ctrl+Shift+L)").clicked() {
                        self.viewer.rotate_all(RotationDelta::CounterClockwise);
                        ui.close();
                    }
                    if ui.button("Rotate Right (Ctrl+Shift+R)").clicked() {
                        self.viewer.rotate_all(RotationDelta::Clockwise);
                        ui.close();
                    }
                    if ui.button("Reset Rotation").clicked() {
                        self.viewer.reset_rotations();
                        ui.close();
                    }
                });
                ui.menu_button("Debug", |ui| {
                    let mut debug = self.viewer.text_layer_debug();
                    if ui.checkbox(&mut debug, "Show Text Layer").clicked() {
                        self.viewer.set_text_layer_debug(debug);
                    }
                });
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About viewkai").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        if self.show_about {
            let mut open = self.show_about;
            let mut should_close = false;
            egui::Window::new("About viewkai")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!("viewkai v{}", env!("CARGO_PKG_VERSION")));
                    ui.label("License: MIT");
                    ui.hyperlink_to("GitHub", "https://github.com/writzx/viewkai");
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });
            self.show_about = open && !should_close;
        }
    }

    fn show_url_window(&mut self, ctx: &egui::Context) {
        if !self.url_dialog.visible {
            return;
        }

        let mut open = self.url_dialog.visible;
        let mut submit = false;
        let mut cancel = false;
        egui::Window::new("Open from URL")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.url_dialog.url_buffer)
                        .desired_width(480.0)
                        .hint_text("https://example.com/file.pdf"),
                );
                if !response.has_focus() {
                    response.request_focus();
                }

                let can_submit = !self.url_dialog.url_buffer.trim().is_empty();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui
                        .add_enabled(can_submit, egui::Button::new("Open"))
                        .clicked()
                    {
                        submit = true;
                    }
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && !self.url_dialog.url_buffer.trim().is_empty()
        {
            submit = true;
        }

        if cancel {
            self.url_dialog.visible = false;
            return;
        }

        if submit {
            let url = self.url_dialog.url_buffer.trim().to_owned();
            self.url_dialog.visible = false;
            self.begin_url_fetch(ctx, url);
            return;
        }

        self.url_dialog.visible = open;
    }
}

impl eframe::App for App {
    // justify: the UI method is the natural egui integration point and grouping
    // its panels in one place keeps event flow easier to audit than splitting it
    // into many tiny helpers.
    #[allow(clippy::too_many_lines)]
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);
        self.poll_pending_load();
        self.sync_viewport_title(&ctx);

        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            self.show_menu_bar(ui);
        });

        egui::Panel::top("app_controls").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                zoom_ui::zoom_toolbar_ui(ui, &mut self.viewer);
                ui.separator();
                ui.label("Mode:");
                self.view_mode_selector_ui(ui);
                ui.separator();
                self.viewer.show_plugin_toolbars(ui);
            });
        });

        egui::Panel::bottom("page_nav").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Page:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.page_input)
                        .desired_width(50.0)
                        .hint_text("1"),
                );
                if self.page_input_focused {
                    response.request_focus();
                    self.page_input_focused = false;
                }
                ui.label(format!("of {}", self.total_pages));

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.jump_to_page_input();
                }
            });
        });

        egui::Panel::bottom("app_debug")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::CollapsingHeader::new("Debug")
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Some(info) = &self.debug_info {
                            ui.label(info);
                        } else {
                            ui.label("No document loaded");
                        }
                    });
            });

        if let Some(active_tab) = self.active_sidebar_tab() {
            egui::Panel::left("viewkai.sidebar")
                .default_size(260.0)
                .resizable(true)
                .show_inside(ui, |ui| {
                    let outline_visible = self.viewer.outline().visible();
                    let thumbnails_visible = self.viewer.thumbnails().visible();

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                outline_visible,
                                egui::Button::new("Outline")
                                    .selected(self.sidebar_tab == SidebarTab::Outline),
                            )
                            .clicked()
                        {
                            self.sidebar_tab = SidebarTab::Outline;
                        }

                        if ui
                            .add_enabled(
                                thumbnails_visible,
                                egui::Button::new("Thumbnails")
                                    .selected(self.sidebar_tab == SidebarTab::Thumbnails),
                            )
                            .clicked()
                        {
                            self.sidebar_tab = SidebarTab::Thumbnails;
                        }
                    });
                    ui.separator();
                    let doc = self.viewer.document_arc();

                    match active_tab {
                        SidebarTab::Outline => {
                            self.viewer.outline_mut().render_panel(ui, doc.as_deref())
                        }
                        SidebarTab::Thumbnails => self
                            .viewer
                            .thumbnails_mut()
                            .render_panel(ui, doc.as_deref()),
                    }
                });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let scroll_delta = ui.input(|i| {
                if i.modifiers.ctrl {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            });

            if scroll_delta.abs() > 0.1 {
                let factor = if scroll_delta > 0.0 {
                    1.1_f32
                } else {
                    1.0 / 1.1
                };
                self.apply_zoom_factor(factor);
            }

            let pinch_delta = ui.input(egui::InputState::zoom_delta);
            if (pinch_delta - 1.0).abs() > 0.01 {
                self.apply_zoom_factor(pinch_delta);
            }

            match &self.load_state {
                LoadState::Loaded | LoadState::Idle => self.viewer.show(ui),
                LoadState::AcquiringBytes { label } => Self::show_loading(ui, label),
                LoadState::Failed { msg } => {
                    let msg = msg.clone();
                    self.show_failure(ui, &msg);
                }
            }
        });

        self.show_about_window(&ctx);
        self.show_url_window(&ctx);
    }
}

fn fetch_url_native(
    url: &str,
    document_name: Option<String>,
    ctx: &egui::Context,
    sink: PendingLoadSink,
) {
    let ctx = ctx.clone();
    let url = url.to_owned();
    ehttp::fetch(ehttp::Request::get(&url), move |res| {
        *sink.lock().unwrap() = Some(
            res.map(|response| PendingLoad {
                bytes: response.bytes.to_vec(),
                source_label: "Processing fetched PDF".to_owned(),
                document_name: document_name.clone(),
            })
            .map_err(|err| err.to_string()),
        );
        ctx.request_repaint();
    });
}

/// Run the native application.
///
/// # Errors
///
/// Returns any `eframe` startup error from creating the native window.
///
/// # Panics
///
/// Panics if `viewkai_engine::init()` cannot initialise `PDFium` for the app.
pub fn run() -> eframe::Result {
    env_logger::init();
    init().expect("Failed to initialize PDFium");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("viewkai")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "viewkai",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
