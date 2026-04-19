use thiserror::Error;

/// All errors produced by the viewkai family of crates.
#[derive(Debug, Error)]
pub enum Error {
    /// A string-typed engine error (used by viewkai-core which cannot depend on viewkai-engine).
    #[error("engine error: {0}")]
    Engine(String),

    /// No document is currently loaded.
    #[error("document not loaded")]
    NotLoaded,

    /// A requested page index exceeded the loaded document range.
    #[error("page index out of range: {0}")]
    PageOutOfRange(usize),

    /// The supplied bytes could not be parsed as a PDF.
    #[error("invalid PDF data")]
    InvalidPdf,
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, Error>;
