//! Web demo application for viewkai.

use eframe::egui;
use std::sync::{Arc, Mutex};
use viewkai::{RotationDelta, ViewMode, Viewer, zoom::ZoomState};
use viewkai_core::PageIndex;
use viewkai_engine::Document;

#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use viewkai_engine::init;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

mod wasm_state;
mod zoom_ui;

#[cfg(target_arch = "wasm32")]
const DEFAULT_PDF: &[u8] = include_bytes!("../../../tests/fixtures/hello.pdf");

pub(crate) type PendingLoadSink = Arc<Mutex<Option<Result<PendingLoad, String>>>>;

pub(crate) struct PendingLoad {
    bytes: Vec<u8>,
    source_label: String,
    document_name: Option<String>,
}

#[derive(Default)]
struct UrlDialog {
    visible: bool,
    url_buffer: String,
}

/// Current loading lifecycle for the web demo application.
pub enum DemoLoadState {
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

/// Web demo application embedding the `viewkai` viewer widget.
pub struct DemoApp {
    viewer: Viewer,
    load_state: DemoLoadState,
    wasm_state: wasm_state::WasmState,
    debug_info: Option<String>,
    page_input: String,
    page_input_focused: bool,
    total_pages: usize,
    sidebar_tab: SidebarTab,
    current_document_name: Option<String>,
    last_viewport_title: String,
    show_about: bool,
    url_dialog: UrlDialog,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Outline,
    Thumbnails,
}

enum LoadEvent {
    BytesReceived(PendingLoad),
    LoadSucceeded,
    LoadFailed {
        message: String,
    },
    Reset,
}

#[derive(Clone, Copy)]
enum ShortcutAction {
    OpenFile,
    OpenUrl,
    CloseDocument,
}

const SHORTCUTS: &[(egui::Modifiers, egui::Key, ShortcutAction)] = &[
    (
        egui::Modifiers::CTRL,
        egui::Key::O,
        ShortcutAction::OpenFile,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::L,
        ShortcutAction::OpenUrl,
    ),
    (
        egui::Modifiers::CTRL,
        egui::Key::W,
        ShortcutAction::CloseDocument,
    ),
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

impl DemoApp {
    /// Create a new web demo app instance.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = cc;

        Self {
            viewer: Viewer::new(),
            load_state: DemoLoadState::Idle,
            wasm_state: wasm_state::WasmState::default(),
            debug_info: None,
            page_input: String::new(),
            page_input_focused: false,
            total_pages: 0,
            sidebar_tab: SidebarTab::Outline,
            current_document_name: None,
            last_viewport_title: String::new(),
            show_about: false,
            url_dialog: UrlDialog::default(),
        }
    }

    fn transition(&mut self, ev: LoadEvent) {
        match ev {
            LoadEvent::BytesReceived(PendingLoad {
                bytes,
                source_label,
                document_name,
            }) => {
                self.load_state = DemoLoadState::AcquiringBytes {
                    label: source_label,
                };
                self.load_bytes(&bytes, document_name);
            }
            LoadEvent::LoadSucceeded => {
                self.load_state = DemoLoadState::Loaded;
            }
            LoadEvent::LoadFailed { message } => {
                self.current_document_name = None;
                self.load_state = DemoLoadState::Failed { msg: message };
            }
            LoadEvent::Reset => {
                self.viewer.clear();
                self.load_state = DemoLoadState::Idle;
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
                self.current_document_name = None;
            }
        }
    }

    /// Load a PDF from bytes without going through the URL loader.
    /// Transitions `DemoLoadState` to `Loaded` on success.
    ///
    /// # Errors
    ///
    /// Returns the user-facing load failure message when parsing or rendering the
    /// provided PDF bytes fails.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_bytes_sync(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.load_bytes(&bytes, None);
        match &self.load_state {
            DemoLoadState::Loaded => Ok(()),
            DemoLoadState::Failed { msg } => Err(msg.clone()),
            _ => Err("unexpected state after load".to_owned()),
        }
    }

    /// Returns a reference to the inner viewer for inspection.
    #[must_use]
    pub fn viewer(&self) -> &Viewer {
        &self.viewer
    }

    /// Returns the current demo loading state.
    #[must_use]
    pub fn load_state(&self) -> &DemoLoadState {
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

    /// Test-only convenience constructor used by harness-based smoke tests.
    pub fn new_for_testing() -> Self {
        Self {
            viewer: Viewer::new(),
            load_state: DemoLoadState::Idle,
            wasm_state: wasm_state::WasmState::default(),
            debug_info: None,
            page_input: String::new(),
            page_input_focused: false,
            total_pages: 0,
            sidebar_tab: SidebarTab::Outline,
            current_document_name: None,
            last_viewport_title: String::new(),
            show_about: false,
            url_dialog: UrlDialog::default(),
        }
    }

    /// Test helper exposing mutable viewer access for smoke tests.
    pub fn viewer_for_testing(&mut self) -> &mut Viewer {
        &mut self.viewer
    }

    /// Test helper forwarding keyboard handling.
    pub fn handle_shortcuts_for_testing(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
    }

    /// Test helper forwarding pending-load polling.
    pub fn poll_pending_load_for_testing(&mut self) {
        self.poll_pending_load();
    }

    /// Test helper forwarding viewport-title sync.
    pub fn sync_viewport_title_for_testing(&mut self, ctx: &egui::Context) {
        self.sync_viewport_title(ctx);
    }

    /// Test helper rendering the menu bar.
    pub fn show_menu_bar_for_testing(&mut self, ui: &mut egui::Ui) {
        self.show_menu_bar(ui);
    }

    /// Test helper rendering the about window.
    pub fn show_about_window_for_testing(&mut self, ctx: &egui::Context) {
        self.show_about_window(ctx);
    }

    /// Test helper rendering the URL dialog.
    pub fn show_url_window_for_testing(&mut self, ctx: &egui::Context) {
        self.show_url_window(ctx);
    }

    /// Test helper rendering the compact top controls row.
    pub fn show_compact_controls_for_testing(&mut self, ui: &mut egui::Ui) {
        zoom_ui::zoom_toolbar_ui(ui, &mut self.viewer);
        ui.separator();
        ui.label("Mode:");
        self.view_mode_selector_ui(ui);
        ui.separator();
        self.viewer.show_plugin_toolbars(ui);
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

    fn start_fetch(&mut self, ctx: &egui::Context, url: String) {
        let pending = Arc::new(Mutex::new(None));
        let pending_clone = Arc::clone(&pending);
        let repaint_ctx = ctx.clone();
        let document_name = Self::document_name_from_url(&url);

        self.load_state = DemoLoadState::AcquiringBytes {
            label: format!("Fetching {url}"),
        };

        ehttp::fetch(ehttp::Request::get(&url), move |result| {
            let bytes = result
                .map(|response| PendingLoad {
                    bytes: response.bytes.to_vec(),
                    source_label: "Processing fetched PDF".to_owned(),
                    document_name: document_name.clone(),
                })
                .map_err(|err| err.to_string());
            *pending_clone.lock().unwrap() = Some(bytes);
            repaint_ctx.request_repaint();
        });

        self.wasm_state.pending_load = Some(pending);
    }

    fn poll_pending_load(&mut self) {
        if let Some(pending) = self.wasm_state.pending_load.as_ref().map(Arc::clone)
            && let Ok(mut guard) = pending.try_lock()
            && let Some(result) = guard.take()
        {
            self.wasm_state.pending_load = None;
            match result {
                Ok(pending) => self.transition(LoadEvent::BytesReceived(pending)),
                Err(message) => self.transition(LoadEvent::LoadFailed { message }),
            }
        }
    }

    fn maybe_load_from_drop(&mut self, ui: &egui::Ui) {
        let dropped_files = ui.input(|input| input.raw.dropped_files.clone());

        for file in dropped_files {
            if let Some(bytes) = file.bytes {
                self.load_bytes(bytes.as_ref(), Some(file.name));
                break;
            }
        }
    }

    fn dismiss_error(&mut self) {
        self.transition(LoadEvent::Reset);
    }

    fn document_name_from_url(url: &str) -> Option<String> {
        let trimmed = url.trim();
        let without_fragment = trimmed.split('#').next().unwrap_or(trimmed);
        let without_query = without_fragment.split('?').next().unwrap_or(without_fragment);
        let name = without_query.rsplit('/').next().unwrap_or(without_query).trim();
        (!name.is_empty()).then(|| name.to_owned())
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

    fn close_document(&mut self) {
        self.transition(LoadEvent::Reset);
    }

    fn show_url_dialog(&mut self) {
        self.url_dialog.visible = true;
    }

    fn open_file(&mut self, ctx: &egui::Context) {
        if let Err(message) = trigger_file_picker(ctx, &mut self.wasm_state.pending_load) {
            self.transition(LoadEvent::LoadFailed { message });
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
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_OUTLINE_TOGGLE)) {
            let is_visible = self.viewer.outline().visible();
            self.toggle_outline_visible(!is_visible);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_THUMBNAILS_TOGGLE)) {
            let is_visible = self.viewer.thumbnails().visible();
            self.toggle_thumbnails_visible(!is_visible);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV_ALT)) {
            self.viewer.prev_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV)) {
            self.viewer.prev_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT_ALT)) {
            self.viewer.next_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT)) {
            self.viewer.next_match();
        }

        for (mods, key, action) in SHORTCUTS {
            let shortcut = egui::KeyboardShortcut::new(*mods, *key);
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                match action {
                    ShortcutAction::OpenFile => self.open_file(ctx),
                    ShortcutAction::OpenUrl => self.show_url_dialog(),
                    ShortcutAction::CloseDocument => self.close_document(),
                }
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

    fn view_mode_label(mode: ViewMode) -> &'static str {
        match mode {
            ViewMode::Single => "Single Page",
            ViewMode::Continuous => "Continuous",
            ViewMode::Spread {
                cover_separate: true,
            } => "Spread (Cover Alone)",
            ViewMode::Spread {
                cover_separate: false,
            } => "Spread (All Pairs)",
        }
    }

    fn view_mode_selector_ui(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("view_mode_combo")
            .selected_text(Self::view_mode_label(self.viewer.view_mode()))
            .show_ui(ui, |ui| {
                for (label, mode) in VIEW_MODE_OPTIONS {
                    if ui.selectable_label(self.viewer.view_mode() == mode, label).clicked() {
                        self.viewer.set_view_mode(mode);
                    }
                }
            });
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
                    self.open_file(ui.ctx());
                }
                if ui.button("Open from URL… (Ctrl+L)").clicked() {
                    ui.close();
                    self.show_url_dialog();
                }
                if ui.button("Close (Ctrl+W)").clicked() {
                    ui.close();
                    self.close_document();
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
                    if ui.checkbox(&mut show_thumbnails, "Show Thumbnails").clicked() {
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
                    if ui.add_enabled(can_submit, egui::Button::new("Open")).clicked() {
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
            self.start_fetch(ctx, url);
            return;
        }

        self.url_dialog.visible = open;
    }
}

impl eframe::App for DemoApp {
    #[allow(clippy::too_many_lines)]
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.handle_shortcuts(&ctx);
        self.poll_pending_load();
        self.sync_viewport_title(&ctx);

        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            self.show_menu_bar(ui);
        });

        egui::Panel::top("web_controls").show_inside(ui, |ui| {
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

        egui::Panel::bottom("web_debug")
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
                        SidebarTab::Thumbnails => {
                            self.viewer.thumbnails_mut().render_panel(ui, doc.as_deref())
                        }
                    }
                });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.maybe_load_from_drop(ui);

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
                DemoLoadState::Loaded | DemoLoadState::Idle => self.viewer.show(ui),
                DemoLoadState::AcquiringBytes { label } => {
                    let label = label.clone();
                    Self::show_loading(ui, &label);
                }
                DemoLoadState::Failed { msg } => {
                    let msg = msg.clone();
                    self.show_failure(ui, &msg);
                }
            }
        });

        self.show_about_window(&ctx);
        self.show_url_window(&ctx);
    }
}

#[cfg(target_arch = "wasm32")]
fn trigger_file_picker(
    ctx: &egui::Context,
    sink_slot: &mut Option<PendingLoadSink>,
) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "missing window".to_owned())?;
    let document = window.document().ok_or_else(|| "missing document".to_owned())?;
    let input = document
        .get_element_by_id("viewkai-file-input")
        .ok_or_else(|| "missing #viewkai-file-input".to_owned())?
        .dyn_into::<web_sys::HtmlInputElement>()
        .map_err(|_| "#viewkai-file-input is not an HtmlInputElement".to_owned())?;

    input.set_value("");
    let sink = Arc::new(Mutex::new(None));
    let pending_sink = Arc::clone(&sink);
    let repaint_ctx = ctx.clone();
    let input_for_listener = input.clone();
    let onchange = Closure::once(move |_event: web_sys::Event| {
        let Some(files) = input_for_listener.files() else {
            return;
        };
        let Some(file) = files.get(0) else {
            return;
        };

        let file_name = file.name();
        let pending_sink = Arc::clone(&pending_sink);
        let repaint_ctx = repaint_ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = async {
                let js_value = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await?;
                let bytes = Uint8Array::new(&js_value).to_vec();
                Ok(PendingLoad {
                    bytes,
                    source_label: format!("Reading {file_name}"),
                    document_name: Some(file_name),
                })
            }
            .await
            .map_err(|err: wasm_bindgen::JsValue| {
                err.as_string()
                    .unwrap_or_else(|| "Failed to read selected file".to_owned())
            });

            *pending_sink.lock().unwrap() = Some(result);
            repaint_ctx.request_repaint();
        });
    });

    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    onchange.forget();
    input.click();
    *sink_slot = Some(sink);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_file_picker(
    _ctx: &egui::Context,
    _sink_slot: &mut Option<PendingLoadSink>,
) -> Result<(), String> {
    Err("Web file picker is only available on wasm32".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(err) = init() {
            web_sys::console::error_1(&format!("viewkai_engine::init failed: {err}").into());
            return;
        }

        let canvas = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("the_canvas_id"))
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("missing canvas#the_canvas_id");

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    let mut app = DemoApp::new(cc);
                    app.load_bytes_sync(DEFAULT_PDF.to_vec())
                        .expect("default hello.pdf bundled with binary should always parse");
                    Ok(Box::new(app))
                }),
            )
            .await
            .expect("Failed to start eframe web runner");
    });
}

#[cfg(not(target_arch = "wasm32"))]
/// Stub for non-WASM builds of the web crate.
pub fn run() {}
