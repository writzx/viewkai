//! WASM-specific state for the web demo application.

use crate::PendingLoadSink;

/// State that only exists on the WASM target.
#[derive(Default)]
pub(crate) struct WasmState {
    pub(crate) pending_load: Option<PendingLoadSink>,
}
