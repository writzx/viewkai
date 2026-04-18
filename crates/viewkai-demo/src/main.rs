#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use viewkai_core::page::PageIndex;
use viewkai_engine::{Document, init};

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
        web_sys::console::warn_1(
            &"initialize_pdfium_render not found, proceeding anyway".into(),
        );
    }

    Ok(())
}

struct DemoApp {
    info: String,
}

impl DemoApp {
    fn new() -> Self {
        Self {
            info: Self::load_pdf_info(),
        }
    }

    fn load_pdf_info() -> String {
        let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();

        match Document::from_bytes(bytes) {
            Ok(doc) => {
                let size = doc
                    .page_size(PageIndex(0))
                    .map(|page| format!("{:.1}x{:.1}", page.width_pt, page.height_pt))
                    .unwrap_or_else(|_| "unknown".to_owned());

                format!(
                    "PDF loaded: {} pages. Page 1 size: {} points.",
                    doc.page_count(),
                    size
                )
            }
            Err(err) => format!("Error loading PDF: {err}"),
        }
    }
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("viewkai demo");
            ui.label(&self.info);
        });
    }
}
