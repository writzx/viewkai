//! Plugin registry — container for a `Viewer`'s registered plugins.

use std::any::TypeId;

use crate::ViewerPlugin;

/// Container for a `Viewer`'s registered plugins.
///
/// Public so the sibling `viewkai` crate can own one on `Viewer`. The
/// **trait** ([`ViewerPlugin`]) is sealed; the **registry** (this type) is not —
/// it merely holds already-constructed sealed instances and dispatches to their
/// hooks.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn ViewerPlugin>>,
}

impl PluginRegistry {
    /// Create a new registry from a list of boxed plugins.
    #[must_use]
    pub fn new(plugins: Vec<Box<dyn ViewerPlugin>>) -> Self {
        Self { plugins }
    }

    /// Iterate mutably over all registered plugins.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Box<dyn ViewerPlugin>> {
        self.plugins.iter_mut()
    }

    /// Iterate over all registered plugins.
    pub fn iter(&self) -> std::slice::Iter<'_, Box<dyn ViewerPlugin>> {
        self.plugins.iter()
    }

    /// Get a shared reference to the first plugin of type `P`, if registered.
    #[must_use]
    pub fn get<P: ViewerPlugin>(&self) -> Option<&P> {
        self.plugins
            .iter()
            .find(|plugin| plugin.as_ref().type_id() == TypeId::of::<P>())
            .and_then(|plugin| {
                let plugin_any = plugin.as_ref() as &dyn std::any::Any;
                plugin_any.downcast_ref::<P>()
            })
    }

    /// Get a mutable reference to the first plugin of type `P`, if registered.
    pub fn get_mut<P: ViewerPlugin>(&mut self) -> Option<&mut P> {
        self.plugins
            .iter_mut()
            .find(|plugin| plugin.as_ref().type_id() == TypeId::of::<P>())
            .and_then(|plugin| {
                let plugin_any = plugin.as_mut() as &mut dyn std::any::Any;
                plugin_any.downcast_mut::<P>()
            })
    }
}

impl<'a> IntoIterator for &'a PluginRegistry {
    type Item = &'a Box<dyn ViewerPlugin>;
    type IntoIter = std::slice::Iter<'a, Box<dyn ViewerPlugin>>;

    fn into_iter(self) -> Self::IntoIter {
        self.plugins.iter()
    }
}

impl<'a> IntoIterator for &'a mut PluginRegistry {
    type Item = &'a mut Box<dyn ViewerPlugin>;
    type IntoIter = std::slice::IterMut<'a, Box<dyn ViewerPlugin>>;

    fn into_iter(self) -> Self::IntoIter {
        self.plugins.iter_mut()
    }
}
