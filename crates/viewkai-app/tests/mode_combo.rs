//! Mode selector regression tests for `viewkai-app`.

mod common;

use eframe::egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use std::sync::{Mutex, OnceLock};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn mode_selector_is_combo_box() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let h = common::demo_harness_with_hello();

    let combos = h.query_all_by_role(Role::ComboBox).collect::<Vec<_>>();

    assert!(
        combos
            .iter()
            .any(|node| node.accesskit_node().value().as_deref() == Some("Continuous")),
        "expected toolbar mode selector to be exposed as a ComboBox"
    );
}
