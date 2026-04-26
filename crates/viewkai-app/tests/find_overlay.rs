//! Find-overlay regression tests.

mod common;

use egui_kittest::kittest::Queryable;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn find_overlay_ascii_fallback() {
    let mut h = common::demo_harness_with_hello();

    h.get_by_label("Find").click();
    h.run_ok();

    assert!(h.get_all_by_label("<").count() >= 2);
    assert!(h.get_all_by_label(">").count() >= 2);
    h.get_by_label("x");
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("▲"))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("▼"))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| h.get_by_label("✕"))).is_err());
}
