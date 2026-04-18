// FILE: ./core/engine/src/logger.rs

use std::collections::VecDeque; // Standard library only, satisfies CC-SOV. [file:2]
use wasm_bindgen::prelude::*; // Build-time JS glue only; no external runtime crates. [file:2]

/// Maximum number of log entries to keep in the ring buffer. [file:2]
const MAX_LOG_ENTRIES: usize = 500;

/// Log level enumeration for structured logging. [file:2]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}

/// A single log entry with timestamp, level, module, and message. [file:2]
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: u64, // milliseconds since epoch
    pub level: LogLevel,
    pub module: String,
    pub message: String,
}

impl LogEntry {
    fn to_json(&self) -> String {
        format!(
            r#"{{"timestamp":{},"level":"{}","module":"{}","message":"{}"}}"#,
            self.timestamp,
            self.level.as_str(),
            escape_json(&self.module),
            escape_json(&self.message)
        )
    }
}

/// Logger struct that stores log entries in a ring buffer. [file:2]
pub struct Logger {
    entries: VecDeque<LogEntry>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// Create a new Logger instance with an empty ring buffer. [file:2]
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_LOG_ENTRIES),
        }
    }

    /// Log a message at the specified level. [file:2]
    pub fn log(&mut self, level: LogLevel, module: &str, message: &str) {
        let entry = LogEntry {
            timestamp: js_timestamp_ms(),
            level,
            module: module.to_string(),
            message: message.to_string(),
        };

        self.entries.push_back(entry);

        // Maintain ring buffer size
        while self.entries.len() > MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
    }

    /// Convenience method for INFO level logs. [file:2]
    pub fn info(&mut self, module: &str, message: &str) {
        self.log(LogLevel::Info, module, message);
    }

    /// Convenience method for WARN level logs. [file:2]
    pub fn warn(&mut self, module: &str, message: &str) {
        self.log(LogLevel::Warn, module, message);
    }

    /// Convenience method for ERROR level logs. [file:2]
    pub fn error(&mut self, module: &str, message: &str) {
        self.log(LogLevel::Error, module, message);
    }

    /// Convenience method for DEBUG level logs. [file:2]
    pub fn debug(&mut self, module: &str, message: &str) {
        self.log(LogLevel::Debug, module, message);
    }

    /// Get all log entries as a JSON array string. [file:2]
    pub fn get_logs_json(&self) -> String {
        let mut out = String::new();
        out.push('[');
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&entry.to_json());
        }
        out.push(']');
        out
    }

    /// Clear all log entries. [file:2]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get the current number of log entries. [file:2]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/* ---------- Global Logger Instance ---------- */ [file:2]

static mut GLOBAL_LOGGER: Option<Logger> = None;

/// Initialize the global logger instance. [file:2]
fn ensure_logger() {
    unsafe {
        if GLOBAL_LOGGER.is_none() {
            GLOBAL_LOGGER = Some(Logger::new());
        }
    }
}

/// Log a message to the global logger. [file:2]
pub fn global_log(level: LogLevel, module: &str, message: &str) {
    unsafe {
        ensure_logger();
        if let Some(logger) = &mut GLOBAL_LOGGER {
            logger.log(level, module, message);
        }
    }
}

/// Get all logs from the global logger as JSON. [file:2]
pub fn global_get_logs_json() -> String {
    unsafe {
        ensure_logger();
        if let Some(logger) = &GLOBAL_LOGGER {
            return logger.get_logs_json();
        }
    }
    "[]".to_string()
}

/// Clear all logs from the global logger. [file:2]
pub fn global_clear_logs() {
    unsafe {
        if let Some(logger) = &mut GLOBAL_LOGGER {
            logger.clear();
        }
    }
}

/* ---------- Helper Functions ---------- */ [file:2]

/// Get current timestamp in milliseconds from JS. [file:2]
fn js_timestamp_ms() -> u64 {
    // This will be called via wasm-bindgen; we use a simple approach
    // In practice, this would call Date.now() via JS shim
    // For now, we return 0 and let the JS side populate timestamps if needed
    0
}

fn escape_json(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/* ---------- WASM Exports for JS Bridge ---------- */ [file:2]

/// Returns all log entries as a JSON array string. [file:2]
#[wasm_bindgen]
pub fn cc_get_logs() -> String {
    global_get_logs_json()
}

/// Clears all log entries. [file:2]
#[wasm_bindgen]
pub fn cc_clear_logs() {
    global_clear_logs();
}

/// Logs an info message from JavaScript. [file:2]
#[wasm_bindgen]
pub fn cc_log_info(module: &str, message: &str) {
    global_log(LogLevel::Info, module, message);
}

/// Logs a warning message from JavaScript. [file:2]
#[wasm_bindgen]
pub fn cc_log_warn(module: &str, message: &str) {
    global_log(LogLevel::Warn, module, message);
}

/// Logs an error message from JavaScript. [file:2]
#[wasm_bindgen]
pub fn cc_log_error(module: &str, message: &str) {
    global_log(LogLevel::Error, module, message);
}

/// Logs a debug message from JavaScript. [file:2]
#[wasm_bindgen]
pub fn cc_log_debug(module: &str, message: &str) {
    global_log(LogLevel::Debug, module, message);
}

/* ---------- Unit Tests ---------- */ [file:2]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_basic_logging() {
        let mut logger = Logger::new();
        logger.info("test_module", "Test info message");
        logger.warn("test_module", "Test warning message");
        logger.error("test_module", "Test error message");

        assert_eq!(logger.len(), 3);
    }

    #[test]
    fn test_logger_ring_buffer_limit() {
        let mut logger = Logger::new();
        
        // Log more than MAX_LOG_ENTRIES
        for i in 0..MAX_LOG_ENTRIES + 100 {
            logger.info("test", &format!("Message {}", i));
        }

        // Should have exactly MAX_LOG_ENTRIES
        assert_eq!(logger.len(), MAX_LOG_ENTRIES);
    }

    #[test]
    fn test_logger_clear() {
        let mut logger = Logger::new();
        logger.info("test", "Message 1");
        logger.info("test", "Message 2");
        
        assert_eq!(logger.len(), 2);
        
        logger.clear();
        
        assert_eq!(logger.len(), 0);
    }

    #[test]
    fn test_log_entry_json() {
        let entry = LogEntry {
            timestamp: 1234567890,
            level: LogLevel::Info,
            module: "test_module".to_string(),
            message: "Test message".to_string(),
        };

        let json = entry.to_json();
        assert!(json.contains("\"timestamp\":1234567890"));
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"module\":\"test_module\""));
        assert!(json.contains("\"message\":\"Test message\""));
    }

    #[test]
    fn test_escape_json_special_chars() {
        assert_eq!(escape_json("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_json("quote\"here"), "quote\\\"here");
        assert_eq!(escape_json("back\\slash"), "back\\\\slash");
        assert_eq!(escape_json("tab\there"), "tab\\there");
    }
}
