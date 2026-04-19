//! PDFium rendering engine for viewkai.
//!
//! # Design
//!
//! This crate wraps `pdfium-render` and exposes a clean, I/O-free API using only
//! viewkai-owned types. `pdfium-render` types never appear in public signatures.
//!
//! ## Initialization
//!
//! Call [`init()`] once per process before using any other function.
//!
//! - **Native:** loads `libpdfium` from the path in `PDFIUM_DYLIB_PATH` env var,
//!   falling back to the vendor path at `<workspace>/vendor/pdfium/{platform}/libpdfium.{ext}`.
//! - **WASM:** expects `initialize_pdfium_render()` already called from JavaScript.

pub const NAME: &str = "viewkai-engine";

pub mod error;

use crate::error::{EngineError, Result};
use pdfium_render::prelude::*;
use std::sync::{Mutex, OnceLock};
use viewkai_core::{
    page::{PageIndex, PageSize},
    render::RawImage,
};

// ── Global pdfium singleton ──────────────────────────────────────────────────

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
static PDFIUM_INIT_LOCK: Mutex<()> = Mutex::new(());

/// Initialise the PDFium library.
///
/// Must be called once before [`Document::from_bytes`]. Safe to call multiple
/// times; subsequent calls are no-ops.
///
/// # Errors
///
/// Returns an error if the pdfium library cannot be found/loaded (native only).
/// On WASM this will error if `initialize_pdfium_render()` has not been called
/// from JavaScript yet.
pub fn init() -> Result<()> {
    if PDFIUM.get().is_some() {
        return Ok(());
    }

    let _guard = PDFIUM_INIT_LOCK
        .lock()
        .map_err(|_| EngineError::InitLockPoisoned)?;

    if PDFIUM.get().is_some() {
        return Ok(());
    }

    let bindings = create_bindings()?;
    let pdfium = Pdfium::new(bindings);
    let _ = PDFIUM.set(pdfium);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn create_bindings() -> Result<Box<dyn PdfiumLibraryBindings>> {
    if let Ok(path) = std::env::var("PDFIUM_DYLIB_PATH") {
        return Pdfium::bind_to_library(&path)
            .map_err(|e| EngineError::BindingsLoad(format!("PDFIUM_DYLIB_PATH={path}: {e}")));
    }

    let vendor_path = vendor_pdfium_path();
    if let Ok(bindings) = Pdfium::bind_to_library(&vendor_path) {
        return Ok(bindings);
    }

    Pdfium::bind_to_system_library()
        .map_err(|e| EngineError::BindingsLoad(format!("system pdfium not found: {e}")))
}

#[cfg(not(target_arch = "wasm32"))]
fn vendor_pdfium_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    #[cfg(target_os = "macos")]
    let sub = "mac-arm64/libpdfium.dylib";
    #[cfg(target_os = "linux")]
    let sub = "linux-x64/libpdfium.so";
    #[cfg(target_os = "windows")]
    let sub = "win-x64/pdfium.dll";
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let sub = "unknown/libpdfium";

    format!("{manifest_dir}/../../vendor/pdfium/{sub}")
}

#[cfg(target_arch = "wasm32")]
fn create_bindings() -> Result<Box<dyn PdfiumLibraryBindings>> {
    Pdfium::bind_to_system_library()
        .map_err(|e| EngineError::BindingsLoad(format!("WASM pdfium not ready: {e}")))
}

// ── Document ─────────────────────────────────────────────────────────────────

/// An opened PDF document.
///
/// Owns the original byte buffer. Page metadata (count and sizes) is parsed at
/// construction time; re-rendering in later stages re-uses the stored bytes.
///
/// All `pdfium-render` types are private to this crate.
pub struct Document {
    /// Original PDF bytes (used by the renderer in later plan stages).
    bytes: Vec<u8>,
    page_count: usize,
    page_sizes: Vec<PageSize>,
}

impl Document {
    /// Open a PDF document from its raw bytes.
    ///
    /// Parses page metadata synchronously. On Plan 01 fixtures (< 50 pages,
    /// < 5 MB) this is sub-100 ms on WASM.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInitialised`] if [`init()`] has not been called,
    /// or [`EngineError::InvalidPdf`] / [`EngineError::Pdfium`] if the bytes
    /// cannot be parsed.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let pdfium = PDFIUM.get().ok_or(EngineError::NotInitialised)?;

        let (count, sizes) = {
            let doc = pdfium
                .load_pdf_from_byte_slice(&bytes, None)
                .map_err(|_| EngineError::InvalidPdf)?;

            let count = doc.pages().len() as usize;
            let sizes = (0..count)
                .map(|i| {
                    let page =
                        doc.pages()
                            .get(i as PdfPageIndex)
                            .map_err(|e| EngineError::Pdfium {
                                message: e.to_string(),
                            })?;
                    Ok(PageSize {
                        width_pt: page.width().value,
                        height_pt: page.height().value,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            (count, sizes)
        };

        Ok(Self {
            bytes,
            page_count: count,
            page_sizes: sizes,
        })
    }

    /// Total number of pages.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Dimensions of the page at `idx` in PDF points.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::PageIndexOutOfBounds`] if `idx` ≥ `page_count()`.
    pub fn page_size(&self, idx: PageIndex) -> Result<PageSize> {
        self.page_sizes
            .get(idx.0)
            .copied()
            .ok_or(EngineError::PageIndexOutOfBounds {
                index: idx.0 as u32,
                count: self.page_count as u32,
            })
    }

    /// Raw PDF bytes — for internal use by the renderer (viewkai crate).
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Render a single page to an RGBA image at the given DPI.
///
/// Opens the document from its stored bytes, renders the page, and returns
/// the raw RGBA pixel buffer. The document is re-opened each call; for
/// repeated rendering, the caller should cache the result.
///
/// # Errors
///
/// Returns [`EngineError::NotInitialised`] if [`init()`] has not been called.
/// Returns [`EngineError::PageIndexOutOfBounds`] if `idx >= doc.page_count()`.
pub fn render_page(doc: &Document, idx: PageIndex, dpi: u32) -> Result<RawImage> {
    let pdfium = PDFIUM.get().ok_or(EngineError::NotInitialised)?;

    let pdf_doc = pdfium
        .load_pdf_from_byte_slice(doc.bytes(), None)
        .map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

    let page = pdf_doc
        .pages()
        .get(idx.0 as PdfPageIndex)
        .map_err(|e| match e {
            PdfiumError::PageIndexOutOfBounds => EngineError::PageIndexOutOfBounds {
                index: idx.0 as u32,
                count: doc.page_count() as u32,
            },
            _ => EngineError::Pdfium {
                message: e.to_string(),
            },
        })?;

    let scale = dpi as f32 / 72.0;
    let width = (page.width().value * scale).round() as Pixels;
    let height = (page.height().value * scale).round() as Pixels;

    let bitmap = page
        .render(width, height, None)
        .map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

    let pixels = bitmap.as_rgba_bytes();

    Ok(RawImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}
