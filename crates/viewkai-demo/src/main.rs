//! Binary entrypoints for the `viewkai-demo` application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    viewkai_demo::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
/// Start the demo from the WASM entrypoint.
pub fn wasm_start() {
    viewkai_demo::run_web();
}
