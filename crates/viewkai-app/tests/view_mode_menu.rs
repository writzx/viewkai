//! View-mode menu tests.

mod common;

use egui_kittest::kittest::Queryable;
use viewkai::ViewMode;

#[test]
fn view_menu_has_view_mode_submenu() {
    let mut h = common::demo_harness_with_500page();
    h.run_ok();

    assert_eq!(h.state().viewer().view_mode(), ViewMode::Continuous);

    h.get_by_label("View").click();
    h.run_ok();
    h.get_by_label("View Mode ⏵");
}
