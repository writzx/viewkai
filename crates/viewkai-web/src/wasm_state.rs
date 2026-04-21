//! WASM-specific state for the web demo application.

use std::sync::{Arc, Mutex};

/// State that only exists on the WASM target.
pub(crate) struct WasmState {
    pub(crate) pending_load: Option<Arc<Mutex<Option<Result<crate::PendingLoad, String>>>>>,
}

impl Default for WasmState {
    fn default() -> Self {
        Self { pending_load: None }
    }
}
