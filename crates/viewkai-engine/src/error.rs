//! Error types for viewkai-engine.
//!
//! `pdfium_render` types are intentionally absent from this module's public
//! surface. All pdfium errors are either mapped to structured variants or
//! stringified into [`EngineError::Pdfium`]. This preserves Invariant 3:
//! pdfium-render types never appear in viewkai-engine's public API.

use thiserror::Error;

/// Errors produced by viewkai-engine operations.
///
/// No variant carries a `pdfium_render::*` type. Known pdfium error cases are
/// mapped to structured variants; unknown cases fall through to
/// [`EngineError::Pdfium`] with a stringified message.
#[derive(Debug, Clone, Error)]
pub enum EngineError {
    /// `PDFium` library not initialised; call [`crate::init()`] first.
    #[error("pdfium not initialised; call viewkai_engine::init() first")]
    NotInitialised,

    /// Failed to load `PDFium` bindings from the given path.
    #[error("failed to load PDFium bindings: {0}")]
    BindingsLoad(String),

    /// The pdfium initialisation lock was poisoned (another thread panicked).
    #[error("pdfium init lock poisoned")]
    InitLockPoisoned,

    /// The byte slice could not be parsed as a valid PDF document.
    #[error("invalid PDF document")]
    InvalidPdf,

    /// Page index out of bounds.
    #[error("page index {index} out of bounds (page count: {count})")]
    PageIndexOutOfBounds {
        /// The requested page index.
        index: u32,
        /// The total number of pages in the document.
        count: u32,
    },

    /// Catch-all for pdfium errors not otherwise classified.
    ///
    /// `message` is `PdfiumError::to_string()`; the underlying type is NOT
    /// wrapped because doing so would leak `pdfium_render::*` through
    /// viewkai-engine's public API (violating Invariant 3).
    #[error("pdfium operation failed: {message}")]
    Pdfium {
        /// Stringified pdfium error message.
        message: String,
    },
}

/// Convenience `Result` alias for viewkai-engine operations.
pub type Result<T> = std::result::Result<T, EngineError>;
