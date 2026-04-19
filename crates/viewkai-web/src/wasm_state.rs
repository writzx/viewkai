#![cfg(target_arch = "wasm32")]
//! WASM-specific state for the web demo application.

use std::sync::{Arc, Mutex};

/// State that only exists on the WASM target.
pub(crate) struct WasmState {
    pub(crate) url_input: String,
    pub(crate) pending_bytes: Option<Arc<Mutex<Option<Result<Vec<u8>, String>>>>>,
}

impl Default for WasmState {
    fn default() -> Self {
        Self {
            url_input: String::new(),
            pending_bytes: None,
        }
    }
}
