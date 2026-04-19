use thiserror::Error;

/// All errors produced by the viewkai family of crates.
#[derive(Debug, Error)]
pub enum Error {
    /// A string-typed engine error (used by viewkai-core which cannot depend on viewkai-engine).
    #[error("engine error: {0}")]
    Engine(String),

    #[error("document not loaded")]
    NotLoaded,

    #[error("page index out of range: {0}")]
    PageOutOfRange(usize),

    #[error("invalid PDF data")]
    InvalidPdf,
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, Error>;
