// FILE .core/engine/src/lib.rs
//
// Code-Command cc-engine1 WASM orchestrator.
// Exposes the CC-API defined in .specs/engine.aln and delegates to internal
// modules: validator, vfs, navigator, taskqueue.
//
// Invariants enforced here:
// - CC-ZERO: no setup/install/env logic; only relative imports.
// - CC-FILE: this header declares the logical path.
// - CC-VOL: multiple concrete functions are exported.
// - CC-LANG: Rust only, no external crates beyond std/wasm-bindgen (build).

use std::cell::RefCell;
use std::collections::HashSet;

use wasm_bindgen::prelude::*;

mod validator;
mod vfs;
mod navigator;
mod taskqueue;

use validator::{ValidationRequest, ValidationResult};
use vfs::{Vfs, VfsSnapshot};
use taskqueue::{TaskQueue, TaskQueuePayload, TaskReport};

thread_local! {
    static VFS_INSTANCE: RefCell<Option<Vfs>> = RefCell::new(None);
    static LAST_ERROR_CODE: RefCell<String> = RefCell::new(String::from("CCENG-000"));
}

/// Internal helper to set the last error code.
fn set_last_error(code: &str) {
    LAST_ERROR_CODE.with(|cell| {
        *cell.borrow_mut() = code.to_string();
    });
}

/// cc_engine_identity
///
/// Returns the EngineIdentity JSON, as specified in ENGINE-IDENTITY-1.
#[wasm_bindgen]
pub fn cc_engine_identity() -> String {
    // This must match ENGINE-IDENTITY-1 in .specs/engine.aln.
    // If any field changes, the spec and tests must be updated together.
    let json = concat!(
        "{",
          "\"id\":\"cc-engine1\",",
          "\"version\":\"1.0.0\",",
          "\"status\":\"active\",",
          "\"languages\":[\"rust\",\"cpp\"],",
          "\"invariants\":[",
            "\"CC-VOL\",",
            "\"CC-LANG\",",
            "\"CC-CRATE\",",
            "\"CC-FILE\",",
            "\"CC-FULL\",",
            "\"CC-DEEP\",",
            "\"CC-ZERO\",",
            "\"CC-PATH\",",
            "\"CC-SOV\",",
            "\"CC-NAV\"",
          "]",
        "}"
    );
    json.to_string()
}

/// cc_engine_capabilities
///
/// Returns a JSON object with engine identity, features, profiles, and tags.
#[wasm_bindgen]
pub fn cc_engine_capabilities() -> String {
    // Feature flags and profiles are currently static; if they become dynamic,
    // this function should be updated accordingly.
    let json = concat!(
        "{",
          "\"engine\":", // inline EngineIdentity
            "{",
              "\"id\":\"cc-engine1\",",
              "\"version\":\"1.0.0\",",
              "\"status\":\"active\",",
              "\"languages\":[\"rust\",\"cpp\"],",
              "\"invariants\":[",
                "\"CC-VOL\",",
                "\"CC-LANG\",",
                "\"CC-CRATE\",",
                "\"CC-FILE\",",
                "\"CC-FULL\",",
                "\"CC-DEEP\",",
                "\"CC-ZERO\",",
                "\"CC-PATH\",",
                "\"CC-SOV\",",
                "\"CC-NAV\"",
              "]",
            "},",
          "\"features\":[",
            "\"token-bench\",",
            "\"connector-sandbox\"",
          "],",
          "\"profiles\":[",
            "\"github\",",
            "\"local\",",
            "\"memory-only\"",
          "],",
          "\"tags\":[",
            "\"CC-VOL\",",
            "\"CC-LANG\",",
            "\"CC-CRATE\",",
            "\"CC-FILE\",",
            "\"CC-FULL\",",
            "\"CC-DEEP\",",
            "\"CC-ZERO\",",
            "\"CC-PATH\",",
            "\"CC-SOV\",",
            "\"CC-NAV\"",
          "]",
        "}"
    );
    json.to_string()
}

/// cc_last_error_code
///
/// Returns the last non-OK error code (string) or "CCENG-000" if none.
#[wasm_bindgen]
pub fn cc_last_error_code() -> String {
    LAST_ERROR_CODE.with(|cell| cell.borrow().clone())
}

/// cc_init_vfs
///
/// Initialize the in-memory VFS from a snapshot JSON. Implements the
/// VFS-SNAPSHOT-1 contract and sets CCENG-010 on version mismatch.
#[wasm_bindgen]
pub fn cc_init_vfs(snapshot_json: &str) -> bool {
    match VfsSnapshot::from_json(snapshot_json) {
        Ok(snapshot) => {
            if snapshot.version != "VFS-SNAPSHOT-1" {
                set_last_error("CCENG-010");
                return false;
            }
            let vfs = Vfs::from_snapshot(snapshot);
            VFS_INSTANCE.with(|cell| {
                *cell.borrow_mut() = Some(vfs);
            });
            set_last_error("CCENG-000");
            true
        }
        Err(_) => {
            // Treat parse failure as snapshot version error for now.
            set_last_error("CCENG-010");
            false
        }
    }
}

/// ccreadfile
///
/// Read a file from the VFS by logical path. If the file does not exist,
/// returns an empty string.
#[wasm_bindgen]
pub fn ccreadfile(path: &str) -> String {
    VFS_INSTANCE.with(|cell| {
        if let Some(vfs) = cell.borrow().as_ref() {
            vfs.read(path).unwrap_or_default()
        } else {
            String::new()
        }
    })
}

/// ccwritefile
///
/// Convenience helper that wraps a single-task SITQ "update" operation.
/// Returns true if the write was accepted by VFS and passed validations.
#[wasm_bindgen]
pub fn ccwritefile(path: &str, content: &str, sha: &str) -> bool {
    // Minimal wrapper: we construct a single-task payload and delegate to
    // cc_execute_task so all invariants are enforced consistently.
    let payload = format!(
        "{{\"version\":\"TASK-LANGUAGE-1.0\",\"tasks\":[{{\
            \"kind\":\"update\",\
            \"path\":\"{}\",\
            \"content\":{},\
            \"sha\":\"{}\",\
            \"tags\":[\"CC-FILE\",\"CC-FULL\",\"CC-LANG\",\"CC-PATH\"]\
        }}]}}",
        escape_json_string(path),
        escape_json_string(content),
        escape_json_string(sha),
    );
    let report_json = cc_execute_task(&payload);
    // Simple heuristic: parse ok flag via string search.
    // For more robustness, the host should inspect TaskReport in detail.
    let ok = report_json.contains("\"ok\":true");
    ok
}

/// cclistdir
///
/// List directory entries under the given path as a JSON array of strings.
#[wasm_bindgen]
pub fn cclistdir(path: &str) -> String {
    VFS_INSTANCE.with(|cell| {
        if let Some(vfs) = cell.borrow().as_ref() {
            let entries = vfs.list(path);
            let mut out = String::from("[");
            let mut first = true;
            for e in entries {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                out.push_str(&escape_json_string(&e));
                out.push('"');
            }
            out.push(']');
            out
        } else {
            "[]".to_string()
        }
    })
}

/// cc_validate_code
///
/// Validate a piece of code under a JSON array of tag IDs.
/// Returns ValidationResult JSON as defined in VALIDATOR-CONTRACT-1.
#[wasm_bindgen]
pub fn cc_validate_code(code: &str, tags_json: &str) -> String {
    let tags = parse_tags_json(tags_json);
    let req = ValidationRequest {
        code: code.to_string(),
        path: String::new(), // path may be inferred from headers elsewhere
        tags,
        previous_symbols: None,
    };
    let result: ValidationResult = validator::run_validation(&req);
    if result.ok {
        set_last_error("CCENG-000");
    } else {
        set_last_error("CCENG-020");
    }
    result.to_json()
}

/// cc_execute_task
///
/// Execute a TaskQueuePayload JSON and return a TaskReport JSON.
/// Implements TASK-LANGUAGE-1.0 and TASK-REPORT-1 contracts.
#[wasm_bindgen]
pub fn cc_execute_task(task_json: &str) -> String {
    let payload = match TaskQueuePayload::from_json(task_json) {
        Ok(p) => p,
        Err(_) => {
            set_last_error("CCENG-001");
            return TaskReport::error("CCENG-001", "Failed to parse task payload").to_json();
        }
    };

    let mut vfs_before: Option<Vfs> = None;

    let report = VFS_INSTANCE.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        let vfs = borrowed.get_or_insert_with(|| Vfs::new_empty());
        // Snapshot for rollback semantics.
        vfs_before = Some(vfs.clone());
        let mut queue = TaskQueue::new(vfs);
        queue.execute(&payload)
    });

    if !report.ok {
        // Roll back to previous snapshot on failure.
        VFS_INSTANCE.with(|cell| {
            if let Some(before) = vfs_before.take() {
                *cell.borrow_mut() = Some(before);
            }
        });
        if report.error_code == Some("CCENG-002".to_string()) {
            set_last_error("CCENG-002");
        } else if report.error_code == Some("CCENG-003".to_string()) {
            set_last_error("CCENG-003");
        } else if report.error_code == Some("CCENG-030".to_string()) {
            set_last_error("CCENG-030");
        } else {
            set_last_error("CCENG-020");
        }
    } else {
        set_last_error("CCENG-000");
    }

    report.to_json()
}

/// Escape a string for inclusion in JSON (minimal subset).
fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Control characters are not expected in our subset; drop them.
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Parse a tags JSON array like ["CC-FILE","CC-LANG"] into a Vec<String>.
fn parse_tags_json(tags_json: &str) -> Vec<String> {
    // Very small hand-rolled parser for the subset we expect.
    let mut tags = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;

    for ch in tags_json.chars() {
        if escape {
            // Only basic escapes are supported; others are treated literally.
            current.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '"' if !in_string => {
                in_string = true;
                current.clear();
            }
            '"' if in_string => {
                in_string = false;
                if !current.is_empty() {
                    tags.push(current.clone());
                }
            }
            '\\' if in_string => {
                escape = true;
            }
            _ if in_string => {
                current.push(ch);
            }
            _ => {}
        }
    }

    tags
}
