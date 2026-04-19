//! Error types for the viewkai viewer crate.

use viewkai_engine::error::EngineError;

/// Errors produced by [`crate::Viewer::load_bytes`].
///
/// Follows Rust API Guidelines C-GOOD-ERR: structured, meaningful variants.
#[derive(Debug, Clone)]
pub enum LoadError {
    /// An error from the underlying `PDFium` engine.
    Engine(EngineError),

    /// The byte slice could not be parsed as a valid PDF.
    InvalidPdf {
        /// Stringified error message.
        source: String,
    },

    /// viewkai-engine not initialised; call `viewkai_engine::init()` first.
    Uninitialised,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(err) => write!(f, "engine error: {err}"),
            Self::InvalidPdf { source } => write!(f, "invalid PDF: {source}"),
            Self::Uninitialised => {
                write!(f, "viewkai-engine not initialised; call viewkai_engine::init() first")
            }
        }
    }
}

impl std::error::Error for LoadError {}
