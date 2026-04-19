//! Registry integration tests — verify built-in plugin ordering and typed access.

use viewkai_plugins::{PluginRegistry, SearchPlugin, TextLayerPlugin, ViewerPlugin};

#[test]
fn registry_exposes_builtins() {
    let registry = PluginRegistry::new(vec![
        Box::new(TextLayerPlugin::new()),
        Box::new(SearchPlugin::new()),
    ]);

    assert_eq!(
        registry.get::<TextLayerPlugin>().map(ViewerPlugin::id),
        Some("viewkai.text_layer")
    );
    assert_eq!(
        registry.get::<SearchPlugin>().map(ViewerPlugin::id),
        Some("viewkai.search")
    );
}

#[test]
fn registry_is_ordered() {
    let registry = PluginRegistry::new(vec![
        Box::new(TextLayerPlugin::new()),
        Box::new(SearchPlugin::new()),
    ]);

    let ids: Vec<&str> = registry.iter().map(|plugin| plugin.id()).collect();
    assert_eq!(ids, vec!["viewkai.text_layer", "viewkai.search"]);
}
