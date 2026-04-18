#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use viewkai::{Viewer, zoom::ZoomState};
use viewkai_core::page::PageIndex;
use viewkai_engine::{Document, init};

const DISCRETE_LEVELS: [f32; 9] = [0.25, 0.50, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
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
            Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    use wasm_bindgen::JsCast;

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
                Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
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

enum DemoLoadState {
    Idle,
    AcquiringBytes { label: String },
    Loaded,
    Failed { msg: String },
}

struct DemoApp {
    viewer: Viewer,
    load_state: DemoLoadState,
    #[cfg(target_arch = "wasm32")]
    url_input: String,
    debug_info: Option<String>,
    page_input: String,
    page_input_focused: bool,
    total_pages: usize,
    #[cfg(target_arch = "wasm32")]
    pending_bytes: Option<Arc<Mutex<Option<Result<Vec<u8>, String>>>>>,
}

impl DemoApp {
    fn new() -> Self {
        Self {
            viewer: Viewer::new(),
            load_state: DemoLoadState::Idle,
            #[cfg(target_arch = "wasm32")]
            url_input: String::new(),
            debug_info: None,
            page_input: String::new(),
            page_input_focused: false,
            total_pages: 0,
            #[cfg(target_arch = "wasm32")]
            pending_bytes: None,
        }
    }

    fn describe_pdf(bytes: &[u8]) -> Result<String, String> {
        let doc = Document::from_bytes(bytes.to_vec()).map_err(|err| err.to_string())?;
        let size = doc
            .page_size(PageIndex(0))
            .map(|page| format!("{:.1}x{:.1}", page.width_pt, page.height_pt))
            .unwrap_or_else(|_| "unknown".to_owned());

        Ok(format!(
            "PDF loaded: {} pages. Page 1 size: {} points.",
            doc.page_count(),
            size
        ))
    }

    fn load_bytes(&mut self, bytes: Vec<u8>) {
        match self.viewer.load_bytes(bytes.clone()) {
            Ok(()) => {
                self.total_pages = self.viewer.page_count();
                self.page_input = if self.total_pages > 0 {
                    "1".to_owned()
                } else {
                    String::new()
                };
                self.debug_info = Some(
                    Self::describe_pdf(&bytes)
                        .unwrap_or_else(|err| format!("PDF loaded; debug info unavailable: {err}")),
                );
                self.load_state = DemoLoadState::Loaded;
            }
            Err(err) => {
                self.debug_info = None;
                self.total_pages = 0;
                self.page_input.clear();
                self.load_state = DemoLoadState::Failed {
                    msg: err.to_string(),
                };
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

        self.load_state = DemoLoadState::AcquiringBytes {
            label: format!("Reading {}", path.display()),
        };

        match std::fs::read(&path) {
            Ok(bytes) => self.load_bytes(bytes),
            Err(err) => {
                self.debug_info = None;
                self.viewer.clear();
                self.load_state = DemoLoadState::Failed {
                    msg: format!("Failed to read {}: {err}", path.display()),
                };
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
            let bytes = result.map(|response| response.bytes).map_err(|err| err.to_string());
            *pending_clone.lock().unwrap() = Some(bytes);
            repaint_ctx.request_repaint();
        });

        self.pending_bytes = Some(pending);
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_pending_bytes(&mut self) {
        if let Some(pending) = self.pending_bytes.as_ref().map(Arc::clone) {
            if let Ok(mut guard) = pending.try_lock() {
                if let Some(result) = guard.take() {
                    self.pending_bytes = None;
                    match result {
                        Ok(bytes) => self.load_bytes(bytes),
                        Err(msg) => {
                            self.debug_info = None;
                            self.viewer.clear();
                            self.load_state = DemoLoadState::Failed { msg };
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
                self.load_bytes(bytes.to_vec());
                break;
            }
        }
    }

    fn dismiss_error(&mut self) {
        self.viewer.clear();
        self.debug_info = None;
        self.total_pages = 0;
        self.page_input.clear();
        self.load_state = DemoLoadState::Idle;
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
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        self.poll_pending_bytes();

        ui.input(|i| {
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::Num0) {
                    self.viewer.set_zoom(ZoomState::Discrete(1.0));
                }
                if i.key_pressed(egui::Key::Num1) {
                    self.viewer.set_zoom(ZoomState::FitWidth);
                }
                if i.key_pressed(egui::Key::Num2) {
                    self.viewer.set_zoom(ZoomState::FitPage);
                }
                if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                    let z = step_zoom_up(self.viewer.zoom());
                    self.viewer.set_zoom(z);
                }
                if i.key_pressed(egui::Key::Minus) {
                    let z = step_zoom_down(self.viewer.zoom());
                    self.viewer.set_zoom(z);
                }
                if i.key_pressed(egui::Key::G) {
                    self.page_input_focused = true;
                }
            }
        });

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

                if ui.button("−").clicked() {
                    let new_zoom = step_zoom_down(self.viewer.zoom());
                    self.viewer.set_zoom(new_zoom);
                }

                let current_label = zoom_label(self.viewer.zoom());
                egui::ComboBox::from_id_salt("zoom_combo")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        for (label, zoom) in zoom_levels() {
                            if ui.selectable_label(false, label).clicked() {
                                self.viewer.set_zoom(zoom);
                            }
                        }
                    });

                if ui.button("+").clicked() {
                    let new_zoom = step_zoom_up(self.viewer.zoom());
                    self.viewer.set_zoom(new_zoom);
                }
            });

            #[cfg(target_arch = "wasm32")]
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.url_input)
                        .hint_text("https://example.com/document.pdf")
                        .desired_width(f32::INFINITY),
                );
                let should_load = ui.button("Load").clicked()
                    || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));

                if should_load {
                    let url = self.url_input.trim().to_owned();
                    if url.is_empty() {
                        self.viewer.clear();
                        self.debug_info = None;
                        self.load_state = DemoLoadState::Failed {
                            msg: "Enter a PDF URL before loading.".to_owned(),
                        };
                    } else {
                        self.start_fetch(ui.ctx(), url);
                    }
                }

                ui.separator();

                if ui.button("−").clicked() {
                    let new_zoom = step_zoom_down(self.viewer.zoom());
                    self.viewer.set_zoom(new_zoom);
                }

                let current_label = zoom_label(self.viewer.zoom());
                egui::ComboBox::from_id_salt("zoom_combo")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        for (label, zoom) in zoom_levels() {
                            if ui.selectable_label(false, label).clicked() {
                                self.viewer.set_zoom(zoom);
                            }
                        }
                    });

                if ui.button("+").clicked() {
                    let new_zoom = step_zoom_up(self.viewer.zoom());
                    self.viewer.set_zoom(new_zoom);
                }
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
                let factor = if scroll_delta > 0.0 { 1.1_f32 } else { 1.0 / 1.1 };
                self.apply_zoom_factor(factor);
            }

            let pinch_delta = ui.input(|i| i.zoom_delta());
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

fn zoom_levels() -> [(&'static str, ZoomState); 11] {
    [
        ("25%", ZoomState::Discrete(0.25)),
        ("50%", ZoomState::Discrete(0.50)),
        ("75%", ZoomState::Discrete(0.75)),
        ("100%", ZoomState::Discrete(1.0)),
        ("125%", ZoomState::Discrete(1.25)),
        ("150%", ZoomState::Discrete(1.50)),
        ("200%", ZoomState::Discrete(2.0)),
        ("300%", ZoomState::Discrete(3.0)),
        ("400%", ZoomState::Discrete(4.0)),
        ("Fit Width", ZoomState::FitWidth),
        ("Fit Page", ZoomState::FitPage),
    ]
}

fn zoom_label(zoom: ZoomState) -> &'static str {
    match zoom {
        ZoomState::Discrete(z) if (z - 0.25).abs() < 0.01 => "25%",
        ZoomState::Discrete(z) if (z - 0.50).abs() < 0.01 => "50%",
        ZoomState::Discrete(z) if (z - 0.75).abs() < 0.01 => "75%",
        ZoomState::Discrete(z) if (z - 1.0).abs() < 0.01 => "100%",
        ZoomState::Discrete(z) if (z - 1.25).abs() < 0.01 => "125%",
        ZoomState::Discrete(z) if (z - 1.5).abs() < 0.01 => "150%",
        ZoomState::Discrete(z) if (z - 2.0).abs() < 0.01 => "200%",
        ZoomState::Discrete(z) if (z - 3.0).abs() < 0.01 => "300%",
        ZoomState::Discrete(z) if (z - 4.0).abs() < 0.01 => "400%",
        ZoomState::FitWidth => "Fit Width",
        ZoomState::FitPage => "Fit Page",
        _ => "Custom",
    }
}

fn step_zoom_up(current: ZoomState) -> ZoomState {
    let z = match current {
        ZoomState::Discrete(z) | ZoomState::Custom(z) => z,
        ZoomState::FitWidth | ZoomState::FitPage => 1.0,
    };
    let next = DISCRETE_LEVELS.iter().find(|&&level| level > z + 0.01).copied();
    ZoomState::Discrete(next.unwrap_or(4.0))
}

fn step_zoom_down(current: ZoomState) -> ZoomState {
    let z = match current {
        ZoomState::Discrete(z) | ZoomState::Custom(z) => z,
        ZoomState::FitWidth | ZoomState::FitPage => 1.0,
    };
    let previous = DISCRETE_LEVELS
        .iter()
        .rev()
        .find(|&&level| level < z - 0.01)
        .copied();
    ZoomState::Discrete(previous.unwrap_or(0.25))
}
