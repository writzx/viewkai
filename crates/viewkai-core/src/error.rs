use thiserror::Error;

/// All errors produced by the viewkai family of crates.
#[derive(Debug, Error)]
pub enum Error {
    #[error("PDFium error: {0}")]
    Pdfium(String),

    #[error("document not loaded")]
    NotLoaded,

    #[error("page index out of range: {0}")]
    PageOutOfRange(usize),

    #[error("invalid PDF data")]
    InvalidPdf,
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, Error>;
