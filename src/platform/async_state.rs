use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::oneshot;

/// Handler name used for postMessage calls across all platforms
pub const HANDLER_NAME: &str = "webdriver_async";

/// Shared state for pending async script operations.
/// This is managed via Tauri's state system (`app.manage()`).
#[derive(Default)]
pub struct AsyncScriptState {
    pending: Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>,
    /// Track which webviews have native handlers registered (by window label)
    registered_handlers: Mutex<HashSet<String>>,
}

impl AsyncScriptState {
    /// Register a pending async operation and return the receiver
    pub fn register(&self, id: String) -> oneshot::Receiver<Result<Value, String>> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut pending) = self.pending.lock() {
            pending.insert(id, tx);
        }
        rx
    }

    /// Complete a pending async operation with a result
    pub fn complete(&self, id: &str, result: Result<Value, String>) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(tx) = pending.remove(id) {
                let _ = tx.send(result);
            }
        }
    }

    /// Cancel a pending async operation
    pub fn cancel(&self, id: &str) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(id);
        }
    }

    /// Check if a handler is registered for a window label, and mark it as registered if not.
    /// Returns true if the handler was already registered, false if it needs to be registered.
    pub fn mark_handler_registered(&self, label: &str) -> bool {
        if let Ok(mut handlers) = self.registered_handlers.lock() {
            !handlers.insert(label.to_string())
        } else {
            false
        }
    }
}
