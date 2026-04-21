//! Built-in plugin smoke tests.

use viewkai_plugins::{OutlinePlugin, SearchPlugin, TextLayerPlugin, ViewerPlugin};

#[test]
fn text_layer_plugin_has_correct_id() {
    let plugin = TextLayerPlugin::new();
    assert_eq!(plugin.id(), "viewkai.text_layer");
}

#[test]
fn search_plugin_has_correct_id() {
    let plugin = SearchPlugin::new();
    assert_eq!(plugin.id(), "viewkai.search");
}

#[test]
fn outline_plugin_has_correct_id() {
    let plugin = OutlinePlugin::new();
    assert_eq!(plugin.id(), "viewkai.outline");
}
