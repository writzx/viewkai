//! Page-navigation regression tests.

mod common;

use egui_kittest::kittest::Queryable;

#[test]
fn prev_next_buttons() {
    let h = common::demo_harness_with_500page();

    h.get_by_label("<");
    h.get_by_label(">");
}

#[test]
fn page_input_auto_update() {
    let mut h = common::demo_harness_with_500page();

    h.state_mut().scroll_to_page_for_testing(4);
    h.run_ok();
    h.run_ok();
    h.run_ok();
    h.run_ok();
    h.run_ok();

    let expected = h
        .state()
        .viewer()
        .visible_pages()
        .first()
        .map_or_else(|| "1".to_owned(), |page| (page.0 + 1).to_string());
    assert_eq!(h.state().page_input_value(), expected);
}
