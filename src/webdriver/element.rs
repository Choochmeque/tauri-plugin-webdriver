use std::collections::HashMap;

use uuid::Uuid;

/// Represents a WebDriver element reference
#[derive(Debug, Clone)]
pub struct ElementRef {
    /// WebDriver element ID (returned to client)
    pub id: String,
    /// JavaScript variable name holding the element reference
    pub js_ref: String,
}

/// Storage for element references within a session
#[derive(Debug, Default)]
pub struct ElementStore {
    elements: HashMap<String, ElementRef>,
    /// Counter for generating unique JS variable names
    counter: u64,
}

impl ElementStore {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            counter: 0,
        }
    }

    /// Store a new element and return its reference
    pub fn store(&mut self) -> ElementRef {
        let id = Uuid::new_v4().to_string();
        let js_ref = format!("__wd_el_{}", self.counter);
        self.counter += 1;

        let elem_ref = ElementRef {
            id: id.clone(),
            js_ref,
        };

        self.elements.insert(id, elem_ref.clone());
        elem_ref
    }

    /// Get element by WebDriver ID
    pub fn get(&self, id: &str) -> Option<&ElementRef> {
        self.elements.get(id)
    }

    /// Remove an element reference
    pub fn remove(&mut self, id: &str) -> bool {
        self.elements.remove(id).is_some()
    }

    /// Clear all stored elements
    pub fn clear(&mut self) {
        self.elements.clear();
        // Don't reset counter to avoid JS variable name collisions
    }

    /// Get the number of stored elements
    pub fn count(&self) -> usize {
        self.elements.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_element() {
        let mut store = ElementStore::new();
        let elem = store.store();

        assert!(!elem.id.is_empty());
        assert_eq!(elem.js_ref, "__wd_el_0");
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_get_element() {
        let mut store = ElementStore::new();
        let elem = store.store();
        let id = elem.id.clone();

        let retrieved = store.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn test_clear_elements() {
        let mut store = ElementStore::new();
        store.store();
        store.store();

        assert_eq!(store.count(), 2);
        store.clear();
        assert_eq!(store.count(), 0);

        // Counter should not reset
        let elem = store.store();
        assert_eq!(elem.js_ref, "__wd_el_2");
    }
}
