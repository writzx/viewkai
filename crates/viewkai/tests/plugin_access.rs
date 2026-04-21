//! Tests for `Viewer`-level typed plugin accessors.

use viewkai::{Viewer, ViewerPlugin};

#[test]
fn typed_accessors_return_correct_ids() {
    let viewer = Viewer::new();
    assert_eq!(viewer.text_layer().id(), "viewkai.text_layer");
    assert_eq!(viewer.search().id(), "viewkai.search");
    assert_eq!(viewer.outline().id(), "viewkai.outline");
    assert_eq!(viewer.thumbnails().id(), "viewkai.thumbnail");
}
