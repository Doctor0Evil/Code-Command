// FILE: ./core/engine/src/log.rs
//! Structured logging ring buffer for cc_poll_logs export.
//! 
//! Implements a lightweight LogRecord struct and global ring buffer
//! that collects structured log events from major engine components.

use core::cell::RefCell;

#[derive(Clone, Debug)]
pub struct LogRecord {
    pub level: String,         // "debug","info","warn","error"
    pub component: String,     // "taskqueue","validator","blacklist","navigator","vfs"
    pub message: String,
    pub correlation_id: String,
    pub timestamp: String,     // ISO8601 string
}

const LOG_CAPACITY: usize = 512;

thread_local! {
    static LOG_BUFFER: RefCell<Vec<LogRecord>> = RefCell::new(Vec::new());
}

pub fn push_log(record: LogRecord) {
    LOG_BUFFER.with(|buf| {
        let mut vec = buf.borrow_mut();
        if vec.len() >= LOG_CAPACITY {
            let drop_count = vec.len() - LOG_CAPACITY + 1;
            vec.drain(0..drop_count);
        }
        vec.push(record);
    });
}

pub fn drain_logs() -> Vec<LogRecord> {
    LOG_BUFFER.with(|buf| {
        let mut vec = buf.borrow_mut();
        let out = vec.clone();
        vec.clear();
        out
    })
}

fn now_iso8601() -> String {
    // Best-effort ISO8601 timestamp using system time fallback
    // In WASM context, JS can provide accurate timestamps via external call
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple epoch-based timestamp (not full ISO8601, but sufficient for ordering)
    format!("{}", secs)
}

pub fn log_info(component: &str, message: &str, correlation_id: &str) {
    push_log(LogRecord {
        level: "info".to_string(),
        component: component.to_string(),
        message: message.to_string(),
        correlation_id: correlation_id.to_string(),
        timestamp: now_iso8601(),
    });
}

pub fn log_warn(component: &str, message: &str, correlation_id: &str) {
    push_log(LogRecord {
        level: "warn".to_string(),
        component: component.to_string(),
        message: message.to_string(),
        correlation_id: correlation_id.to_string(),
        timestamp: now_iso8601(),
    });
}

pub fn log_error(component: &str, message: &str, correlation_id: &str) {
    push_log(LogRecord {
        level: "error".to_string(),
        component: component.to_string(),
        message: message.to_string(),
        correlation_id: correlation_id.to_string(),
        timestamp: now_iso8601(),
    });
}
