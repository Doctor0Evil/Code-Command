// FILE: ./core/engine/src/lib.rs

#![allow(clippy::unused_unit)] // Kept minimal; no external lints. [file:2]

mod validator;
mod vfs;
mod task_queue;

use validator::{ValidationRequest, ValidationResult};
use vfs::Vfs;
use task_queue::TaskQueue;

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
}

/// Validates a code artifact against the requested CC- tags.
/// `code` is the full file contents, `tags_json` is a JSON array of tag IDs. [file:2]
#[wasm_bindgen]
pub fn cc_validate(code: &str, tags_json: &str) -> JsValue {
    let req = ValidationRequest::from_json(code, tags_json);
    let result: ValidationResult = validator::run_validation(req);
    JsValue::from_str(&result.to_json())
}

/// Reads a file from the virtual file system after path normalization.
/// Returns the file contents or an empty string on failure. [file:2]
#[wasm_bindgen]
pub fn cc_read_file(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.read(path).unwrap_or_default();
        }
    }
    String::new()
}

/// Writes a file into the virtual file system after enforcing CC-PATH and CC-DEEP.
/// `sha` is used by the JS layer for GitHub concurrency control. [file:2]
#[wasm_bindgen]
pub fn cc_write_file(path: &str, content: &str, sha: &str) -> bool {
    unsafe {
        if let Some(vfs) = &mut VFS {
            return vfs.write(path, content, sha);
        }
    }
    false
}

/// Lists the contents of a directory path as a JSON string (array of entries).
/// The exact JSON shape is defined in specs/api.aln. [file:2]
#[wasm_bindgen]
pub fn cc_list_dir(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.list(path);
        }
    }
    "[]".to_string()
}

/// Executes a Single-Iteration Task Queue (SITQ) payload over the VFS.
/// `task_json` encodes a list of file operations and validation requests. [file:2]
#[wasm_bindgen]
pub fn cc_execute_task(task_json: &str) -> String {
    let mut queue = TaskQueue::from_json(task_json);

    unsafe {
        if let Some(vfs) = &mut VFS {
            let report = queue.execute(vfs);
            return report.to_json();
        }
    }

    // If VFS is not initialized, return a failure report that the caller can display. [file:2]
    TaskQueue::empty_failure("VFS not initialized").to_json()
}
