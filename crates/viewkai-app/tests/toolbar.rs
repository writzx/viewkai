//! Toolbar regression tests.

mod common;

use egui_kittest::kittest::Queryable;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn no_toolbar_checkboxes() {
    let h = common::demo_harness_with_hello();

    h.get_by_label("Find");
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("Show Outline"))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("Show Thumbnails"))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("Show text layer"))).is_err());
}
