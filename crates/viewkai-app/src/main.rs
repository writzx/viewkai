//! Binary entrypoint for the `viewkai-app` native application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    viewkai_app::run()
}
