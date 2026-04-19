//! Demo application wiring for native and WASM `viewkai` embeds.

use eframe::egui;
use viewkai::{Viewer, zoom::ZoomState};
use viewkai_core::PageIndex;
use viewkai_engine::{Document, init};

mod zoom_ui;
#[cfg(target_arch = "wasm32")]
mod wasm_state;

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Current loading lifecycle for the demo application.
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

/// Example application embedding the `viewkai` viewer widget.
pub struct DemoApp {
    viewer: Viewer,
    load_state: DemoLoadState,
    #[cfg(target_arch = "wasm32")]
    wasm_state: wasm_state::WasmState,
    debug_info: Option<String>,
    page_input: String,
    page_input_focused: bool,
    total_pages: usize,
}

enum LoadEvent {
    BytesReceived { bytes: Vec<u8>, source_label: String },
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
    FocusPageInput,
}

const SHORTCUTS: &[(egui::Modifiers, egui::Key, ShortcutAction)] = &[
    (egui::Modifiers::CTRL, egui::Key::Num0, ShortcutAction::ResetZoom),
    (egui::Modifiers::CTRL, egui::Key::Num1, ShortcutAction::FitWidth),
    (egui::Modifiers::CTRL, egui::Key::Num2, ShortcutAction::FitPage),
    (egui::Modifiers::CTRL, egui::Key::Plus, ShortcutAction::ZoomIn),
    (egui::Modifiers::CTRL, egui::Key::Equals, ShortcutAction::ZoomIn),
    (egui::Modifiers::CTRL, egui::Key::Minus, ShortcutAction::ZoomOut),
    (
        egui::Modifiers::CTRL,
        egui::Key::G,
        ShortcutAction::FocusPageInput,
    ),
];

impl DemoApp {
    /// Create a new demo app instance.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = cc;

        Self {
            viewer: Viewer::new(),
            load_state: DemoLoadState::Idle,
            #[cfg(target_arch = "wasm32")]
            wasm_state: wasm_state::WasmState::default(),
            debug_info: None,
            page_input: String::new(),
            page_input_focused: false,
            total_pages: 0,
        }
    }

    fn transition(&mut self, ev: LoadEvent) {
        match ev {
            LoadEvent::BytesReceived {
                bytes,
                source_label,
            } => {
                self.load_state = DemoLoadState::AcquiringBytes {
                    label: source_label,
                };
                self.load_bytes(&bytes);
            }
            LoadEvent::LoadSucceeded => {
                self.load_state = DemoLoadState::Loaded;
            }
            LoadEvent::LoadFailed { message } => {
                self.load_state = DemoLoadState::Failed { msg: message };
            }
            LoadEvent::Reset => {
                self.viewer.clear();
                self.load_state = DemoLoadState::Idle;
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
            }
        }
    }

    /// Load a PDF from bytes without going through the file dialog.
    /// Transitions `DemoLoadState` to `Loaded` on success.
    ///
    /// # Errors
    ///
    /// Returns the user-facing load failure message when parsing or rendering the
    /// provided PDF bytes fails.
    // justify: the demo test/helpers hand owned fixture buffers through this API
    // and retaining `Vec<u8>` avoids a public signature churn for little benefit.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_bytes_sync(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.load_bytes(&bytes);
        match &self.load_state {
            DemoLoadState::Loaded => Ok(()),
            DemoLoadState::Failed { msg } => Err(msg.clone()),
            _ => Err("unexpected state after load".to_owned()),
        }
    }

    /// Returns a reference to the inner viewer for inspection.
    #[must_use]
    pub fn viewer(&self) -> &viewkai::Viewer {
        &self.viewer
    }

    /// Returns the current demo loading state.
    #[must_use]
    pub fn load_state(&self) -> &DemoLoadState {
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

    #[cfg(not(target_arch = "wasm32"))]
    fn open_file(&mut self) {
        self.load_state = DemoLoadState::AcquiringBytes {
            label: "Opening file…".to_owned(),
        };

        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        else {
            self.load_state = DemoLoadState::Idle;
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

    #[cfg(target_arch = "wasm32")]
    fn start_fetch(&mut self, ctx: &egui::Context, url: String) {
        let pending = Arc::new(Mutex::new(None));
        let pending_clone = Arc::clone(&pending);
        let repaint_ctx = ctx.clone();

        self.load_state = DemoLoadState::AcquiringBytes {
            label: format!("Fetching {url}"),
        };

        ehttp::fetch(ehttp::Request::get(&url), move |result| {
            let bytes = result
                .map(|response| response.bytes)
                .map_err(|err| err.to_string());
            *pending_clone.lock().unwrap() = Some(bytes);
            repaint_ctx.request_repaint();
        });

        self.wasm_state.pending_bytes = Some(pending);
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_pending_bytes(&mut self) {
        if let Some(pending) = self.wasm_state.pending_bytes.as_ref().map(Arc::clone) {
            if let Ok(mut guard) = pending.try_lock() {
                if let Some(result) = guard.take() {
                    self.wasm_state.pending_bytes = None;
                    match result {
                        Ok(bytes) => self.transition(LoadEvent::BytesReceived {
                            bytes,
                            source_label: "Processing fetched PDF".to_owned(),
                        }),
                        Err(msg) => {
                            self.debug_info = None;
                            self.viewer.clear();
                            self.transition(LoadEvent::LoadFailed { message: msg });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn maybe_load_from_drop(&mut self, ui: &egui::Ui) {
        let dropped_files = ui.input(|input| input.raw.dropped_files.clone());

        for file in dropped_files {
            if let Some(bytes) = file.bytes {
                self.load_bytes(bytes.as_ref());
                break;
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
            ShortcutAction::FocusPageInput => self.page_input_focused = true,
        }
    }
}

impl eframe::App for DemoApp {
    // justify: the UI method is the natural egui integration point and grouping
    // its panels in one place keeps event flow easier to audit than splitting it
    // into many tiny helpers.
    #[allow(clippy::too_many_lines)]
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        self.poll_pending_bytes();

        self.handle_shortcuts(ui.ctx());

        egui::Panel::top("demo_controls").show_inside(ui, |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open…").clicked() {
                        ui.close();
                        self.open_file();
                    }
                });

                ui.separator();

                zoom_ui::zoom_toolbar_ui(ui, &mut self.viewer);
            });

            #[cfg(target_arch = "wasm32")]
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.wasm_state.url_input)
                        .hint_text("https://example.com/document.pdf")
                        .desired_width(f32::INFINITY),
                );
                let should_load = ui.button("Load").clicked()
                    || (response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)));

                if should_load {
                    let url = self.wasm_state.url_input.trim().to_owned();
                    if url.is_empty() {
                        self.viewer.clear();
                        self.debug_info = None;
                        self.transition(LoadEvent::LoadFailed {
                            message: "Enter a PDF URL before loading.".to_owned(),
                        });
                    } else {
                        self.start_fetch(ui.ctx(), url);
                    }
                }

                ui.separator();

                zoom_ui::zoom_toolbar_ui(ui, &mut self.viewer);
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

        egui::Panel::bottom("demo_debug")
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

        egui::CentralPanel::default().show_inside(ui, |ui| {
            #[cfg(target_arch = "wasm32")]
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
                DemoLoadState::AcquiringBytes { label } => Self::show_loading(ui, label),
                DemoLoadState::Failed { msg } => {
                    let msg = msg.clone();
                    self.show_failure(ui, &msg);
                }
                DemoLoadState::Idle | DemoLoadState::Loaded => self.viewer.show(ui),
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Run the native demo application.
///
/// # Errors
///
/// Returns any `eframe` startup error from creating the native window.
///
/// # Panics
///
/// Panics if `viewkai_engine::init()` cannot initialise `PDFium` for the demo.
pub fn run_native() -> eframe::Result {
    env_logger::init();
    init().expect("Failed to initialize PDFium");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("viewkai demo")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "viewkai-demo",
        options,
        Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
/// Run the WebAssembly demo application.
pub fn run_web() {
    wasm_bindgen_futures::spawn_local(async {
        wait_for_pdfium_module().await;

        if let Err(err) = call_initialize_pdfium_render() {
            web_sys::console::error_1(&format!("pdfium init failed: {err}").into());
            return;
        }

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
                Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))),
            )
            .await
            .expect("Failed to start eframe web runner");
    });
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_pdfium_module() {
    loop {
        let ready = web_sys::window()
            .and_then(|window| js_sys::Reflect::get(&window, &"__pdfiumModule".into()).ok())
            .map(|value| !value.is_null() && !value.is_undefined())
            .unwrap_or(false);

        if ready {
            break;
        }

        let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED);
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

#[cfg(target_arch = "wasm32")]
fn call_initialize_pdfium_render() -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| "no window".to_owned())?;
    let pdfium_module = js_sys::Reflect::get(&window, &"__pdfiumModule".into())
        .map_err(|err| format!("missing __pdfiumModule: {err:?}"))?;

    let global = js_sys::global();
    let wasm_bindgen = js_sys::Reflect::get(&global, &"wasm_bindgen".into())
        .map_err(|err| format!("missing wasm_bindgen global: {err:?}"))?;
    let get_module = js_sys::Reflect::get(&wasm_bindgen, &"__wbindgen_get_module".into())
        .map_err(|err| format!("missing __wbindgen_get_module: {err:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "__wbindgen_get_module is not a function".to_owned())?;
    let local_module = get_module
        .call0(&wasm_bindgen::JsValue::UNDEFINED)
        .map_err(|err| format!("__wbindgen_get_module call failed: {err:?}"))?;

    let init_fn = js_sys::Reflect::get(&wasm_bindgen, &"initialize_pdfium_render".into())
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok());

    if let Some(function) = init_fn {
        function
            .call3(
                &wasm_bindgen::JsValue::UNDEFINED,
                &pdfium_module,
                &local_module,
                &false.into(),
            )
            .map_err(|err| format!("initialize_pdfium_render call failed: {err:?}"))?;
    } else {
        web_sys::console::warn_1(&"initialize_pdfium_render not found, proceeding anyway".into());
    }

    Ok(())
}
