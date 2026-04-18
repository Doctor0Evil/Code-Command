// FILE: ./core/engine/src/lib.rs

#![allow(clippy::unused_unit)] // Kept minimal; no external lints. [file:2]

mod validator;
mod vfs;
mod task_queue;
mod logger;
mod github_fallback;

use validator::{ValidationRequest, ValidationResult};
use vfs::Vfs;
use task_queue::TaskQueue;
use logger::{global_log, LogLevel};

use wasm_bindgen::prelude::*; // Build-time JS glue only per design. [file:2]

/// Global VFS instance for the WASM engine. In a real build this will be
/// wired to JS shims that talk to the GitHub API. [file:2]
static mut VFS: Option<Vfs> = None;

/// Initializes the in-memory VFS once per session.
/// JS can call this to seed the engine with a snapshot of the repo tree. [file:2]
#[wasm_bindgen]
pub fn cc_init_vfs(serialized_vfs_json: &str) {
    let vfs = Vfs::from_json(serialized_vfs_json);
    unsafe {
        VFS = Some(vfs);
    }
    global_log(LogLevel::Info, "cc_init_vfs", "VFS initialized successfully");
}

/// Validates a code artifact against the requested CC- tags.
/// `code` is the full file contents, `tags_json` is a JSON array of tag IDs. [file:2]
#[wasm_bindgen]
pub fn cc_validate(code: &str, tags_json: &str) -> JsValue {
    let req = ValidationRequest::from_json(code, tags_json);
    global_log(LogLevel::Debug, "cc_validate", &format!("Validating with tags: {}", tags_json));
    let result: ValidationResult = validator::run_validation(req);
    if result.ok {
        global_log(LogLevel::Info, "cc_validate", "Validation passed");
    } else {
        global_log(LogLevel::Warn, "cc_validate", &format!("Validation failed: {} failures", result.failures.len()));
    }
    JsValue::from_str(&result.to_json())
}

/// Reads a file from the virtual file system after path normalization.
/// Returns the file contents or an empty string on failure. [file:2]
#[wasm_bindgen]
pub fn cc_read_file(path: &str) -> String {
    global_log(LogLevel::Debug, "cc_read_file", &format!("Reading file: {}", path));
    unsafe {
        if let Some(vfs) = &VFS {
            let content = vfs.read(path).unwrap_or_default();
            if content.is_empty() {
                global_log(LogLevel::Warn, "cc_read_file", &format!("File not found or empty: {}", path));
            }
            return content;
        }
    }
    global_log(LogLevel::Error, "cc_read_file", "VFS not initialized");
    String::new()
}

/// Writes a file into the virtual file system after enforcing CC-PATH and CC-DEEP.
/// `sha` is used by the JS layer for GitHub concurrency control. [file:2]
#[wasm_bindgen]
pub fn cc_write_file(path: &str, content: &str, sha: &str) -> bool {
    global_log(LogLevel::Debug, "cc_write_file", &format!("Writing file: {}", path));
    unsafe {
        if let Some(vfs) = &mut VFS {
            let result = vfs.write(path, content, sha);
            if result {
                global_log(LogLevel::Info, "cc_write_file", &format!("File written successfully: {}", path));
            } else {
                global_log(LogLevel::Error, "cc_write_file", &format!("Failed to write file: {}", path));
            }
            return result;
        }
    }
    global_log(LogLevel::Error, "cc_write_file", "VFS not initialized");
    false
}

/// Lists the contents of a directory path as a JSON string (array of entries).
/// The exact JSON shape is defined in specs/api.aln. [file:2]
#[wasm_bindgen]
pub fn cc_list_dir(path: &str) -> String {
    global_log(LogLevel::Debug, "cc_list_dir", &format!("Listing directory: {}", path));
    unsafe {
        if let Some(vfs) = &VFS {
            let result = vfs.list(path);
            global_log(LogLevel::Info, "cc_list_dir", &format!("Listed {} entries", result.matches("path").count()));
            return result;
        }
    }
    global_log(LogLevel::Error, "cc_list_dir", "VFS not initialized");
    "[]".to_string()
}

/// Executes a Single-Iteration Task Queue (SITQ) payload over the VFS.
/// `task_json` encodes a list of file operations and validation requests. [file:2]
#[wasm_bindgen]
pub fn cc_execute_task(task_json: &str) -> String {
    global_log(LogLevel::Info, "cc_execute_task", "Starting task execution");
    let mut queue = TaskQueue::from_json(task_json);

    unsafe {
        if let Some(vfs) = &mut VFS {
            let report = queue.execute(vfs);
            if report.ok {
                global_log(LogLevel::Info, "cc_execute_task", "Task execution completed successfully");
            } else {
                global_log(LogLevel::Error, "cc_execute_task", "Task execution failed");
            }
            return report.to_json();
        }
    }

    // If VFS is not initialized, return a failure report that the caller can display. [file:2]
    global_log(LogLevel::Error, "cc_execute_task", "VFS not initialized");
    TaskQueue::empty_failure("VFS not initialized").to_json()
}
