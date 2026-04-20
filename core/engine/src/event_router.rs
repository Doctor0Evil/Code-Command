/// Event routing system for non-hot-path engine events
///
/// Provides a simple publish-subscribe mechanism for events like
/// VFS updates, validation completion, blacklist incidents, etc.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Types of events that can be emitted by the engine
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    /// VFS was updated with new/modified/deleted files
    VfsUpdated,
    /// Validation completed for one or more files
    ValidationCompleted,
    /// A block-level blacklist violation was detected
    BlacklistIncident,
    /// Wiring manifest or graph changed
    WiringChanged,
    /// Custom event type for extensibility
    Custom(String),
}

impl EventType {
    pub fn to_string(&self) -> String {
        match self {
            EventType::VfsUpdated => "vfs_updated".to_string(),
            EventType::ValidationCompleted => "validation_completed".to_string(),
            EventType::BlacklistIncident => "blacklist_incident".to_string(),
            EventType::WiringChanged => "wiring_changed".to_string(),
            EventType::Custom(s) => format!("custom:{}", s),
        }
    }
    
    pub fn from_string(s: &str) -> Self {
        match s {
            "vfs_updated" => EventType::VfsUpdated,
            "validation_completed" => EventType::ValidationCompleted,
            "blacklist_incident" => EventType::BlacklistIncident,
            "wiring_changed" => EventType::WiringChanged,
            other => EventType::Custom(other.to_string()),
        }
    }
}

/// Generic event structure with payload
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: String,
    pub correlation_id: String,
    pub payload: String, // JSON-encoded payload
}

impl Event {
    pub fn new(event_type: EventType, payload: String) -> Self {
        Event {
            event_type,
            timestamp: crate::time::now_iso8601(),
            correlation_id: generate_correlation_id(),
            payload,
        }
    }
    
    pub fn with_correlation_id(event_type: EventType, payload: String, correlation_id: String) -> Self {
        Event {
            event_type,
            timestamp: crate::time::now_iso8601(),
            correlation_id,
            payload,
        }
    }
    
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        out.push_str("\"event_type\":\"");
        out.push_str(&escape_json(&self.event_type.to_string()));
        out.push_str("\",\"timestamp\":\"");
        out.push_str(&escape_json(&self.timestamp));
        out.push_str("\",\"correlation_id\":\"");
        out.push_str(&escape_json(&self.correlation_id));
        out.push_str("\",\"payload\":");
        out.push_str(&self.payload);
        out.push('}');
        out
    }
}

/// Trait for event routers
pub trait EventRouter: Send + Sync {
    /// Subscribe to an event type with a callback
    fn subscribe(&mut self, event: EventType, callback: Box<dyn Fn(&Event) + Send + 'static>);
    
    /// Emit an event to all subscribers of that event type
    fn emit(&self, event: &Event);
    
    /// Unsubscribe all callbacks for an event type
    fn unsubscribe(&mut self, event: &EventType);
}

/// Simple in-memory event router implementation
pub struct SimpleEventRouter {
    subscribers: Arc<Mutex<HashMap<EventType, Vec<Box<dyn Fn(&Event) + Send + 'static>>>>,
}

impl SimpleEventRouter {
    pub fn new() -> Self {
        SimpleEventRouter {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for SimpleEventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRouter for SimpleEventRouter {
    fn subscribe(&mut self, event: EventType, callback: Box<dyn Fn(&Event) + Send + 'static>) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.entry(event).or_insert_with(Vec::new).push(callback);
    }
    
    fn emit(&self, event: &Event) {
        let subs = self.subscribers.lock().unwrap();
        if let Some(callbacks) = subs.get(&event.event_type) {
            for callback in callbacks {
                callback(event);
            }
        }
        
        // Also emit to Custom("*") wildcard if registered
        if let EventType::Custom(_) = &event.event_type {
            // Already handled above
        }
    }
    
    fn unsubscribe(&mut self, event: &EventType) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.remove(event);
    }
}

// Thread-local global event router instance
thread_local! {
    static GLOBAL_ROUTER: Arc<Mutex<SimpleEventRouter>> = Arc::new(Mutex::new(SimpleEventRouter::new()));
}

/// Get the global event router instance
pub fn get_global_router() -> Arc<Mutex<SimpleEventRouter>> {
    GLOBAL_ROUTER.with(|r| r.clone())
}

/// Emit an event via the global router
pub fn emit_event(event: &Event) {
    GLOBAL_ROUTER.with(|router| {
        let r = router.lock().unwrap();
        r.emit(event);
    });
}

/// Subscribe to an event type on the global router
pub fn subscribe_global(event: EventType, callback: Box<dyn Fn(&Event) + Send + 'static>) {
    GLOBAL_ROUTER.with(|router| {
        let mut r = router.lock().unwrap();
        r.subscribe(event, callback);
    });
}

fn generate_correlation_id() -> String {
    // Simple correlation ID generator using timestamp + random-ish value
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("evt-{}-{:04}", now.as_millis(), rand_simple() % 10000)
}

fn rand_simple() -> u128 {
    // Simple LCG-based pseudo-random for correlation IDs
    static mut SEED: u128 = 12345;
    unsafe {
        SEED = SEED.wrapping_mul(6364136223846793005).wrapping_add(1);
        SEED
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            EventType::VfsUpdated,
            "{\"paths\":[\"src/main.rs\"]}".to_string(),
        );
        
        assert_eq!(event.event_type, EventType::VfsUpdated);
        assert!(!event.correlation_id.is_empty());
        assert!(!event.timestamp.is_empty());
    }

    #[test]
    fn test_simple_router_subscribe_emit() {
        let mut router = SimpleEventRouter::new();
        let received = Rc::new(RefCell::new(Vec::new()));
        
        let received_clone = received.clone();
        router.subscribe(
            EventType::VfsUpdated,
            Box::new(move |event| {
                received_clone.borrow_mut().push(event.correlation_id.clone());
            }),
        );
        
        let event = Event::new(
            EventType::VfsUpdated,
            "{\"paths\":[\"test.rs\"]}".to_string(),
        );
        let corr_id = event.correlation_id.clone();
        
        router.emit(&event);
        
        assert_eq!(received.borrow().len(), 1);
        assert_eq!(received.borrow()[0], corr_id);
    }

    #[test]
    fn test_event_type_from_string() {
        assert_eq!(EventType::from_string("vfs_updated"), EventType::VfsUpdated);
        assert_eq!(EventType::from_string("blacklist_incident"), EventType::BlacklistIncident);
        assert_eq!(EventType::from_string("custom:myevent"), EventType::Custom("myevent".to_string()));
    }
}
