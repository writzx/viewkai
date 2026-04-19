//! `PDFium` rendering engine for viewkai.
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

/// Canonical crate name for the rendering engine crate.
pub const NAME: &str = "viewkai-engine";

pub mod error;

use crate::error::{EngineError, Result};
use pdfium_render::prelude::*;
use std::{
    pin::Pin,
    sync::{Mutex, OnceLock},
};
use viewkai_core::{
    page::{PageIndex, PageSize},
    render::RawImage,
};

// ── Global pdfium singleton ──────────────────────────────────────────────────

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
static PDFIUM_INIT_LOCK: Mutex<()> = Mutex::new(());

/// Initialise the `PDFium` library.
///
/// Must be called once before [`Document::from_bytes`]. Safe to call multiple
/// times; subsequent calls are no-ops.
///
/// # Errors
///
/// Returns an error if the `PDFium` library cannot be found or loaded (native only).
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
/// Owns the original byte buffer and the live `PdfDocument`. Page metadata
/// (count and sizes) is parsed at construction time; later rendering re-uses
/// the cached document instead of re-parsing the PDF bytes.
///
/// All `pdfium-render` types are private to this crate.
pub struct Document {
    /// SAFETY CRITICAL — see SAFETY comment on `Document::from_bytes`.
    /// `pdf` MUST be declared first: Rust drops fields in declaration order,
    /// so `pdf` drops before `bytes`, ensuring the backing memory is still
    /// valid when `PdfDocument`'s destructor runs.
    pdf: PdfDocument<'static>,
    /// Heap-pinned owner of the raw PDF bytes. Must outlive `pdf` above.
    /// Never accessed directly after `Document::from_bytes` returns.
    bytes: Pin<Box<[u8]>>,
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
    // justify: `pdfium-render` exposes page counts and page indices as signed
    // integers; these conversions stay within range because they originate from
    // `doc.pages().len()` and are reused only while iterating that exact count.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let pdfium = PDFIUM.get().ok_or(EngineError::NotInitialised)?;

        // Pin the bytes on the heap so their address is stable for the lifetime
        // of this `Document`.
        let bytes: Pin<Box<[u8]>> = Pin::new(bytes.into_boxed_slice());

        let pdf: PdfDocument<'static> = {
            let loaded = pdfium
                .load_pdf_from_byte_slice(&bytes, None)
                .map_err(|_| EngineError::InvalidPdf)?;

            // SAFETY:
            // 1. `PdfDocument<'a>` is tied to the `Pdfium` bindings lifetime. The
            //    bindings live in `static PDFIUM: OnceLock<Pdfium>`, so the bindings
            //    side of the borrow graph is process-lifetime and outlives every
            //    `Document` created by this crate.
            //
            // 2. The byte-slice loader also ties `'a` to the provided byte
            //    slice. We satisfy that requirement by first moving the bytes into
            //    `Pin<Box<[u8]>>`. The heap allocation stays at a stable address even
            //    if the outer `Document` moves, so Pdfium never observes relocated
            //    backing storage.
            //
            // 3. The pinned byte owner is stored in the same `Document` as the
            //    widened `PdfDocument<'static>`. We never mutate, resize, or replace
            //    that boxed slice after the document is opened, so the memory stays
            //    valid for the full logical lifetime of `pdf`.
            //
            // 4. Field order is load-bearing: `pdf` is declared before `bytes`, and
            //    Rust drops struct fields in declaration order. `pdf` therefore drops
            //    before `bytes`, so any final Pdfium reads during `PdfDocument::drop()`
            //    still see valid backing memory. Reordering the fields would break
            //    this safety argument and would be unsound.
            //
            // 5. No widened `'static` borrow escapes the public API. The transmute is
            //    used only so this self-referential storage pattern compiles; callers
            //    can access page data only through borrows derived from `&self`, which
            //    cannot outlive the owning `Document`.
            unsafe {
                std::mem::transmute::<
                    PdfDocument<'_>,
                    PdfDocument<'static>,
                >(loaded)
            }
        };

        let count = pdf.pages().len() as usize;
        let sizes = (0..count)
            .map(|i| {
                let page = pdf.pages().get(i as PdfPageIndex).map_err(|e| EngineError::Pdfium {
                    message: e.to_string(),
                })?;
                Ok(PageSize {
                    width_pt: page.width().value,
                    height_pt: page.height().value,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            pdf,
            bytes,
            page_count: count,
            page_sizes: sizes,
        })
    }

    fn render_config() -> &'static Mutex<PdfRenderConfig> {
        static CFG: OnceLock<Mutex<PdfRenderConfig>> = OnceLock::new();

        CFG.get_or_init(|| Mutex::new(PdfRenderConfig::new()))
    }

    /// Total number of pages.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Dimensions of the page at `idx` in PDF points.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::PageIndexOutOfBounds`] if `idx` ≥ `page_count()`.
    // justify: `EngineError::PageIndexOutOfBounds` stores `u32` values and the
    // engine's `usize` counts are constrained by `pdfium-render` page indexing.
    #[allow(clippy::cast_possible_truncation)]
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
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Render a single page to an RGBA image at the given DPI.
///
/// Reuses the opened document, renders the page, and returns the raw RGBA
/// pixel buffer.
///
/// # Errors
///
/// Returns [`EngineError::PageIndexOutOfBounds`] if `idx >= doc.page_count()`.
// justify: `pdfium-render`'s render API requires signed pixel/index types and
// the conversion points are bounded by `PDFium` page sizes and caller-provided
// DPI values in the viewer's fixed zoom buckets.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn render_page(doc: &Document, idx: PageIndex, dpi: u32) -> Result<RawImage> {
    let page = doc
        .pdf
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

    let mut shared_config = Document::render_config()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let config = std::mem::replace(&mut *shared_config, PdfRenderConfig::new())
        .set_target_width(width)
        .set_target_height(height);
    drop(shared_config);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| EngineError::Pdfium {
            message: e.to_string(),
        })?;

    let mut shared_config = Document::render_config()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *shared_config = config;

    let pixels = bitmap.as_rgba_bytes();

    Ok(RawImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}
