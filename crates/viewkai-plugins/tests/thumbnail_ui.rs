//! Thumbnail UI regression tests.

use std::sync::{Mutex, OnceLock};

use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::{NodeT, Queryable}};
use viewkai_engine::Document;
use viewkai_plugins::ThumbnailPlugin;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn no_caption_label_rendered() {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    viewkai_engine::init().expect("pdfium init");

    let bytes = include_bytes!("../../../tests/fixtures/hello.pdf").to_vec();
    let doc = Document::from_bytes(bytes).expect("load hello.pdf");
    let mut harness = Harness::builder().build_ui_state(
        |ui, plugin: &mut ThumbnailPlugin| plugin.render_panel(ui, Some(&doc)),
        ThumbnailPlugin::new(),
    );
    harness.run_ok();

    let caption_labels = harness
        .query_all_by_role(Role::Label)
        .filter_map(|node| node.accesskit_node().label())
        .filter(|label| {
            label
                .strip_prefix("Page ")
                .is_some_and(|suffix| suffix.parse::<usize>().is_ok())
        })
        .collect::<Vec<_>>();

    assert!(
        caption_labels.is_empty(),
        "expected no visible caption labels, got {caption_labels:?}"
    );
}
