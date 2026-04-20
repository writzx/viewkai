//! Binary entrypoints for the `viewkai-web` WASM application.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("viewkai-web is a WASM-only crate. Use `viewkai-app` for native.");
    std::process::exit(1);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    let _keep_export: fn() = viewkai_web::run;
}
