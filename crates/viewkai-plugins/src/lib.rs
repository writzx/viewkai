//! Sealed plugin abstraction for viewkai. Only built-in plugin types
//! implemented inside this crate may implement [`ViewerPlugin`]. The
//! [`PluginRegistry`] type is public so that the sibling `viewkai` crate
//! can own an instance of it on `Viewer`; sealing protects the trait,
//! not the registry container.

#![warn(missing_docs)]

mod outline;
mod plugin;
mod registry;
mod search;
mod thumbnail;
mod text_layer;

pub use outline::OutlinePlugin;
pub use plugin::{PluginContext, PointerEvent, ViewerPlugin};
pub use registry::PluginRegistry;
pub use search::SearchPlugin;
pub use thumbnail::ThumbnailPlugin;
pub use text_layer::TextLayerPlugin;

pub(crate) mod sealed {
    /// Private supertrait that prevents external types from implementing
    /// [`super::ViewerPlugin`]. Only types inside this crate can implement
    /// `Sealed` and therefore `ViewerPlugin`.
    pub trait Sealed {}
}

/// Optional test-support surface (activate with the `test-support` cargo
/// feature in downstream test crates). Exposes a [`PluginContext`] builder
/// for unit tests that need to drive plugin hooks without a real viewer.
///
/// Not part of the library's supported public API.
#[cfg(feature = "test-support")]
pub mod test_support;
