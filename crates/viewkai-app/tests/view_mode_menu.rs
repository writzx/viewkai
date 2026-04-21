//! View-mode menu tests.

mod common;

use egui_kittest::kittest::Queryable;
use viewkai::ViewMode;

#[test]
fn view_menu_single_radio_sets_mode() {
    let mut h = common::demo_harness_with_500page();
    h.run_ok();

    assert_eq!(h.state().viewer().view_mode(), ViewMode::Continuous);

    h.get_by_label("Single Page").click();
    h.run_ok();

    assert_eq!(h.state().viewer().view_mode(), ViewMode::Single);
}
