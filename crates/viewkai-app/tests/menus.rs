//! Menu-structure regression tests.

mod common;

use egui_kittest::kittest::Queryable;

#[test]
fn sidebar_menu_entries_exist() {
    let mut h = common::demo_harness_with_hello();

    h.get_by_label("View").click();
    h.run_ok();
    h.get_by_label("Sidebar ⏵").click();
    h.run_ok();

    h.get_by_label("Show Outline");
    h.get_by_label("Show Thumbnails");
}
