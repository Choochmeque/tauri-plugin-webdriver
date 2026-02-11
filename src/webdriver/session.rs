use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use super::element::ElementStore;

/// Session timeouts configuration
#[derive(Debug, Clone, Serialize)]
pub struct Timeouts {
    /// Implicit wait timeout in milliseconds
    pub implicit_ms: u64,
    /// Page load timeout in milliseconds
    pub page_load_ms: u64,
    /// Script execution timeout in milliseconds
    pub script_ms: u64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            implicit_ms: 0,
            page_load_ms: 300_000,
            script_ms: 30_000,
        }
    }
}

/// Represents a WebDriver session
#[derive(Debug)]
pub struct Session {
    /// Unique session identifier
    pub id: String,
    /// Session timeouts
    pub timeouts: Timeouts,
    /// Element reference storage
    pub elements: ElementStore,
    /// Current window handle
    pub current_window: String,
}

impl Session {
    pub fn new(initial_window: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timeouts: Timeouts::default(),
            elements: ElementStore::new(),
            current_window: initial_window,
        }
    }
}

/// Manages WebDriver sessions
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new session
    pub fn create(&mut self, initial_window: String) -> &Session {
        let session = Session::new(initial_window);
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get(&id).expect("session was just inserted")
    }

    /// Get a session by ID
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// Delete a session
    pub fn delete(&mut self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }
}
