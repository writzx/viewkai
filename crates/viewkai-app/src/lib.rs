//! Native PDF viewer application built on `viewkai`.

use eframe::egui;
use viewkai::{Viewer, zoom::ZoomState};
use viewkai_core::PageIndex;
use viewkai_engine::{Document, init};

mod zoom_ui;

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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Outline,
    Thumbnails,
}

enum LoadEvent {
    BytesReceived {
        bytes: Vec<u8>,
        source_label: String,
    },
    LoadSucceeded,
    LoadFailed {
        message: String,
    },
    Reset,
}

enum ShortcutAction {
    ResetZoom,
    FitWidth,
    FitPage,
    ZoomIn,
    ZoomOut,
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
        }
    }

    fn transition(&mut self, ev: LoadEvent) {
        match ev {
            LoadEvent::BytesReceived {
                bytes,
                source_label,
            } => {
                self.load_state = LoadState::AcquiringBytes {
                    label: source_label,
                };
                self.load_bytes(&bytes);
            }
            LoadEvent::LoadSucceeded => {
                self.load_state = LoadState::Loaded;
            }
            LoadEvent::LoadFailed { message } => {
                self.load_state = LoadState::Failed { msg: message };
            }
            LoadEvent::Reset => {
                self.viewer.clear();
                self.load_state = LoadState::Idle;
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
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
        self.load_bytes(&bytes);
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

    fn load_bytes(&mut self, bytes: &[u8]) {
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
                self.transition(LoadEvent::LoadSucceeded);
            }
            Err(err) => {
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
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
            Ok(bytes) => self.transition(LoadEvent::BytesReceived {
                bytes,
                source_label: format!("Reading {}", path.display()),
            }),
            Err(err) => {
                self.debug_info = None;
                self.viewer.clear();
                self.transition(LoadEvent::LoadFailed {
                    message: format!("Failed to read {}: {err}", path.display()),
                });
            }
        }
    }

    fn dismiss_error(&mut self) {
        self.transition(LoadEvent::Reset);
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
        for (mods, key, action) in SHORTCUTS {
            let shortcut = egui::KeyboardShortcut::new(*mods, *key);
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                self.apply_shortcut_action(action);
            }
        }

        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV_ALT)) {
            self.viewer.prev_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT_ALT)) {
            self.viewer.next_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_PREV)) {
            self.viewer.prev_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_FIND_NEXT)) {
            self.viewer.next_match();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_OUTLINE_TOGGLE)) {
            let is_visible = self.viewer.outline().visible();
            let new_visible = !is_visible;
            self.viewer.outline_mut().set_visible(new_visible);
            if new_visible {
                self.sidebar_tab = SidebarTab::Outline;
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SHORTCUT_THUMBNAILS_TOGGLE)) {
            let is_visible = self.viewer.thumbnails().visible();
            let new_visible = !is_visible;
            self.viewer.thumbnails_mut().set_visible(new_visible);
            if new_visible {
                self.sidebar_tab = SidebarTab::Thumbnails;
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

    fn apply_shortcut_action(&mut self, action: &ShortcutAction) {
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
        }
    }
}

impl eframe::App for App {
    // justify: the UI method is the natural egui integration point and grouping
    // its panels in one place keeps event flow easier to audit than splitting it
    // into many tiny helpers.
    #[allow(clippy::too_many_lines)]
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ui.ctx());

        egui::Panel::top("app_controls").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open…").clicked() {
                        ui.close();
                        self.open_file();
                    }
                });

                ui.separator();

                zoom_ui::zoom_toolbar_ui(ui, &mut self.viewer);
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
                        SidebarTab::Thumbnails => {
                            self.viewer.thumbnails_mut().render_panel(ui, doc.as_deref())
                        }
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
    }
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
            .with_title("viewkai-app")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "viewkai-app",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
