//! Binary entrypoints for the `viewkai-web` WASM application.

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
/// Start the web demo from the WASM entrypoint.
pub fn start() {
    viewkai_web::run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("viewkai-web is a WASM-only crate. Use `viewkai-app` for native.");
    std::process::exit(1);
}

#[cfg(target_arch = "wasm32")]
fn main() {}
