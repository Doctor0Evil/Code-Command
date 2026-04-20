// FILE: ./core/engine/src/lib.rs

#![allow(clippy::unused_unit)]

// Core modules (all relative, satisfying CC-ZERO).
mod validator;
mod vfs;
mod taskqueue;
mod logger;
mod github_fallback;
mod tokenwalker;
mod blacklist;
mod wiring;

use validator::{ValidationRequest, ValidationResult};
use vfs::{Vfs, VirtualFileSystem, CC_VFS_ID};
use taskqueue::TaskQueue;
use logger::{global_log, LogLevel};
use tokenwalker::{TokenWalker, build_scan_mask};
use blacklist::{BlacklistScanProfile, blacklist_matches_to_json};
use wiring::get_engine;

// wasm-bindgen is allowed as build-time glue only per design.
use wasm_bindgen::prelude::*;

// Global VFS instance for the WASM engine.
// Safety: accessed only inside unsafe blocks in exported functions.
static mut VFS: Option<Vfs> = None;

//
// Canonical cc-* WASM exports (short names, no logging).
// These map directly onto the CC-API surface described in specs/api.aln.
//

/// Mount an in-memory VFS snapshot (VFS-SNAPSHOT-1 JSON).
#[wasm_bindgen]
pub fn ccinitvfs(serialized_vfs_json: &str) {
    let vfs = Vfs::from_json(serialized_vfs_json);
    unsafe {
        VFS = Some(vfs);
    }
}

/// Read a file from the virtual file system.
/// Returns an empty string on failure.
#[wasm_bindgen]
pub fn ccreadfile(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.read(path).unwrap_or_default();
        }
    }
    String::new()
}

/// Write a file into the virtual file system.
/// Returns true on success, false on failure.
#[wasm_bindgen]
pub fn ccwritefile(path: &str, content: &str, sha: &str) -> bool {
    unsafe {
        if let Some(vfs) = &mut VFS {
            return vfs.write(path, content, sha);
        }
    }
    false
}

/// List directory contents as JSON (array of entries).
#[wasm_bindgen]
pub fn cclistdir(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.list(path);
        }
    }
    "[]".to_string()
}

/// Run a validation pass over a code buffer with the given tag list.
/// Returns a JSON-encoded ValidationResult.
#[wasm_bindgen]
pub fn ccvalidatecode(code: &str, tags_json: &str) -> JsValue {
    let req = ValidationRequest::from_json(code, tags_json);
    let result: ValidationResult = validator::run_validation(req);
    JsValue::from_str(&result.to_json())
}

/// Execute a Single-Iteration Task Queue (SITQ) payload over the VFS.
/// Returns a JSON-encoded TaskReport.
#[wasm_bindgen]
pub fn ccexecutetask(task_json: &str) -> String {
    let mut queue = TaskQueue::from_json(task_json);

    unsafe {
        if let Some(vfs) = &mut VFS {
            let report = queue.execute(vfs);
            return report.to_json();
        }
    }

    TaskQueue::empty_failure("VFS not initialized.").to_json()
}

/// Report the active cc-vfs identity string.
///
/// JS and CI can call this to assert that the engine is exposing the
/// canonical cc-vfs implementation and version.
#[wasm_bindgen]
pub fn ccvfs_id() -> String {
    CC_VFS_ID.to_string()
}

//
// Blacklist-only scan export.
// This uses the same blacklist profile as the validator, but does not
// perform full validation or emit a ValidationResult.
//

/// Scan a code buffer against the active blacklist profile only.
///
/// `code` is the buffer to scan.
/// `profile_json` is a small JSON object that encodes language and tags, e.g.:
/// `{ "language": "rust", "tags": ["CC-FULL", "CC-SOV"] }`
///
/// Returns a JSON array of BlacklistMatch objects.
#[wasm_bindgen]
pub fn cc_scan_blacklist(code: &str, profile_json: &str) -> String {
    // 1. Parse profile_json into a small struct.
    let profile = BlacklistScanProfile::from_json(profile_json);

    // 2. Build ScanProfile bitmask with blacklist scanning enabled.
    let scan_mask = build_scan_mask(&profile.tags, profile.language, true);

    // 3. Get blacklist profile from WiringManifest / ENGINE.
    let engine = get_engine().expect("engine not initialized");
    let blacklist = &engine.validator.blacklist_profile;

    // 4. Run token walker in blacklist-only mode.
    let walker = TokenWalker::new(code, scan_mask, profile.language);
    let matches = walker.scan_blacklist_only(blacklist);

    // 5. Serialize Vec<BlacklistMatch> to JSON array and return.
    blacklist_matches_to_json(&matches)
}

//
// Verbose cc_* exports with structured logging.
// These wrap the same core behaviors but emit LogEvents for debugging
// and observability.
//

/// Initializes the in-memory VFS once per session.
/// JS can call this to seed the engine with a snapshot of the repo tree.
#[wasm_bindgen]
pub fn cc_init_vfs(serialized_vfs_json: &str) {
    let vfs = Vfs::from_json(serialized_vfs_json);

    unsafe {
        VFS = Some(vfs);
    }

    global_log(
        LogLevel::Info,
        "cc_init_vfs",
        "VFS initialized successfully",
    );
}

/// Validates a code artifact against the requested CC- tags.
/// `code` is the full file contents, `tags_json` is a JSON array of tag IDs.
#[wasm_bindgen]
pub fn cc_validate(code: &str, tags_json: &str) -> JsValue {
    let req = ValidationRequest::from_json(code, tags_json);

    global_log(
        LogLevel::Debug,
        "cc_validate",
        &format!("Validating with tags: {}", tags_json),
    );

    let result: ValidationResult = validator::run_validation(req);

    if result.ok {
        global_log(LogLevel::Info, "cc_validate", "Validation passed");
    } else {
        global_log(
            LogLevel::Warn,
            "cc_validate",
            &format!("Validation failed: {} failures", result.failures.len()),
        );
    }

    JsValue::from_str(&result.to_json())
}

/// Reads a file from the virtual file system after path normalization.
/// Returns the file contents or an empty string on failure.
#[wasm_bindgen]
pub fn cc_read_file(path: &str) -> String {
    global_log(
        LogLevel::Debug,
        "cc_read_file",
        &format!("Reading file: {}", path),
    );

    unsafe {
        if let Some(vfs) = &VFS {
            let content = vfs.read(path).unwrap_or_default();

            if content.is_empty() {
                global_log(
                    LogLevel::Warn,
                    "cc_read_file",
                    &format!("File not found or empty: {}", path),
                );
            }

            return content;
        }
    }

    global_log(LogLevel::Error, "cc_read_file", "VFS not initialized");

    // Optionally, a future github_fallback::try_read could be called here.
    String::new()
}

/// Writes a file into the virtual file system after enforcing CC-PATH and CC-DEEP.
/// `sha` is used by the JS layer for GitHub concurrency control.
#[wasm_bindgen]
pub fn cc_write_file(path: &str, content: &str, sha: &str) -> bool {
    global_log(
        LogLevel::Debug,
        "cc_write_file",
        &format!("Writing file: {}", path),
    );

    unsafe {
        if let Some(vfs) = &mut VFS {
            let result = vfs.write(path, content, sha);

            if result {
                global_log(
                    LogLevel::Info,
                    "cc_write_file",
                    &format!("File written successfully: {}", path),
                );
            } else {
                global_log(
                    LogLevel::Error,
                    "cc_write_file",
                    &format!("Failed to write file: {}", path),
                );
            }

            return result;
        }
    }

    global_log(LogLevel::Error, "cc_write_file", "VFS not initialized");
    false
}

/// Lists the contents of a directory path as a JSON string (array of entries).
/// The exact JSON shape is defined in specs/api.aln.
#[wasm_bindgen]
pub fn cc_list_dir(path: &str) -> String {
    global_log(
        LogLevel::Debug,
        "cc_list_dir",
        &format!("Listing directory: {}", path),
    );

    unsafe {
        if let Some(vfs) = &VFS {
            let result = vfs.list(path);

            // Crude metric: count occurrences of `"path"` in the JSON string.
            let count = result.matches("\"path\"").count();
            global_log(
                LogLevel::Info,
                "cc_list_dir",
                &format!("Listed {} entries", count),
            );

            return result;
        }
    }

    global_log(LogLevel::Error, "cc_list_dir", "VFS not initialized");
    "[]".to_string()
}

/// Executes a Single-Iteration Task Queue (SITQ) payload over the VFS.
/// `task_json` encodes a list of file operations and validation requests.
#[wasm_bindgen]
pub fn cc_execute_task(task_json: &str) -> String {
    global_log(LogLevel::Info, "cc_execute_task", "Starting task execution");

    let mut queue = TaskQueue::from_json(task_json);

    unsafe {
        if let Some(vfs) = &mut VFS {
            let report = queue.execute(vfs);

            if report.ok {
                global_log(
                    LogLevel::Info,
                    "cc_execute_task",
                    "Task execution completed successfully",
                );
            } else {
                global_log(
                    LogLevel::Error,
                    "cc_execute_task",
                    "Task execution failed",
                );
            }

            return report.to_json();
        }
    }

    global_log(LogLevel::Error, "cc_execute_task", "VFS not initialized");
    TaskQueue::empty_failure("VFS not initialized").to_json()
}
