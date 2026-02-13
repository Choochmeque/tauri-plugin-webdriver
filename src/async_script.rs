use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use tauri::{command, State};
use tokio::sync::oneshot;

/// Shared state for pending async script operations.
/// When an async script callback fires, it invokes a Tauri command
/// which sends the result through the corresponding channel.
#[derive(Default)]
pub struct AsyncScriptState {
    pending: Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>,
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

    /// Cancel a pending async operation (e.g., on timeout)
    pub fn cancel(&self, id: &str) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(id);
        }
    }
}

/// Tauri command called by JavaScript when an async script completes
#[command]
pub async fn resolve(
    state: State<'_, AsyncScriptState>,
    id: String,
    result: Option<Value>,
    error: Option<String>,
) -> Result<(), ()> {
    let outcome = match error {
        Some(e) if !e.is_empty() => Err(e),
        _ => Ok(result.unwrap_or(Value::Null)),
    };
    state.complete(&id, outcome);
    Ok(())
}
