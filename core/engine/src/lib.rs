// FILE: ./core/engine/src/lib.rs

#![allow(clippy::unused_unit)]

// Core modules (all relative, satisfying CC-ZERO).
mod validator_plugin;
mod vfs;
mod task_queue;
mod logger;
mod github_fallback;
mod blacklist;
mod blacklist_cache;
mod blacklist_pattern;
mod wiring_graph;
mod wiring_validator;
mod log;
mod path;
mod cache_key;
mod blacklist_diff;
mod blacklist_summary;
mod event_router;
mod navigator;
mod capacity_engine;
mod capacity_specs;
mod tests_engine_contract;

use validator_plugin::{ValidationRequest, ValidationResult};
use vfs::{Vfs, VirtualFileSystem, CC_VFS_ID};
use task_queue::TaskQueue;
use logger::{global_log, LogLevel};
use blacklist::{BlacklistScanProfile, blacklist_matches_to_json};
use blacklist_cache::{BlacklistCache, BlacklistCacheEntry, BlacklistMatch};
use blacklist_diff::{diff_rules, BlacklistDiff};
use blacklist_summary::BlacklistSummary;
use event_router::{Event, EventType, emit_event, get_global_router};
use wiring_graph::WiringGraph;
use wiring_validator::WiringValidator;
use log::{drain_logs, LogRecord, log_info, log_warn, log_error};

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
    let result: ValidationResult = validator_plugin::run_validation(req);
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

    let result: ValidationResult = validator_plugin::run_validation(req);

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

/// Returns all pending log records as a JSON array and clears the buffer.
/// JS should poll this periodically and dispatch to OutputPanel.
#[wasm_bindgen]
pub fn cc_poll_logs() -> String {
    let records = drain_logs();
    logs_to_json(&records)
}

fn logs_to_json(records: &[LogRecord]) -> String {
    let mut out = String::new();
    out.push('[');
    for (i, r) in records.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_kv_str(&mut out, "level", &r.level);
        out.push(',');
        push_json_kv_str(&mut out, "component", &r.component);
        out.push(',');
        push_json_kv_str(&mut out, "message", &r.message);
        out.push(',');
        push_json_kv_str(&mut out, "correlation_id", &r.correlation_id);
        out.push(',');
        push_json_kv_str(&mut out, "timestamp", &r.timestamp);
        out.push('}');
    }
    out.push(']');
    out
}

fn push_json_kv_str(out: &mut String, key: &str, val: &str) {
    out.push('"');
    out.push_str(key);
    out.push('"');
    out.push(':');
    out.push('"');
    for ch in val.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Returns the wiring graph as compact JSON for external tools.
#[wasm_bindgen]
pub fn cc_get_wiring_json() -> String {
    let engine = match get_engine() {
        Some(e) => e,
        None => return "{}".to_string(),
    };

    let manifest = &engine.wiring;
    let graph = manifest.to_graph();
    graph.to_json()
}

/// Benchmarks the token walker with optional blacklist scanning.
/// profile_json: { "language":"rust", "tags":["CC-VOL","CC-SOV"], "want_blacklist": true }
/// Returns JSON with time_ms_base, time_ms_with_blacklist, throughput_line, 
/// throughput_byte, symbol_count, blacklist_hits.
#[wasm_bindgen]
pub fn cc_bench_token_walker(code: &str, profile_json: &str) -> String {
    use crate::tokenwalker::{LanguageHint, ScanProfile};
    use crate::blacklist::BlacklistProfile;

    // Parse profile_json manually
    let mut language = LanguageHint::Rust;
    let mut tags: Vec<String> = Vec::new();
    let mut want_blacklist = false;

    for raw in profile_json.split(|c| c == '{' || c == '}' || c == '[' || c == ']' || c == ',') {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("\"language\"") {
            if let Some(idx) = line.find(':') {
                let val = line[idx + 1..].trim().trim_matches('"');
                language = match val {
                    "rust" => LanguageHint::Rust,
                    "js" | "javascript" => LanguageHint::Js,
                    "cpp" | "cxx" | "c++" => LanguageHint::Cpp,
                    "aln" => LanguageHint::Aln,
                    "md" | "markdown" => LanguageHint::Md,
                    _ => LanguageHint::Rust,
                };
            }
        } else if line.starts_with("\"want_blacklist\"") {
            if let Some(idx) = line.find(':') {
                let val = line[idx + 1..].trim();
                want_blacklist = val.starts_with('t') || val.starts_with("true");
            }
        } else if line.starts_with('"') {
            let val = line.trim_matches('"');
            if !val.contains(':') && !val.eq_ignore_ascii_case("language") && !val.eq_ignore_ascii_case("want_blacklist") {
                tags.push(val.to_string());
            }
        }
    }

    let total_bytes = code.as_bytes().len() as f64;
    let total_lines = code.lines().count() as f64;

    // Baseline pass without blacklist
    let start = js_now_ms();
    let mask_base = build_scan_mask(&tags, language, false);
    let mut walker = TokenWalker::new(code, mask_base, language);
    let symbols = walker.collect_symbols();
    let end = js_now_ms();
    let time_ms_base = (end - start) as f64;
    let symbol_count = symbols.len() as u32;

    let mut time_ms_with_blacklist = time_ms_base;
    let mut blacklist_hits = 0u32;

    if want_blacklist {
        let engine = match get_engine() {
            Some(e) => e,
            None => return "{}".to_string(),
        };
        let blacklist: &BlacklistProfile = &engine.validator.blacklist_profile;
        let mask_bl = build_scan_mask(&tags, language, true);

        let start_bl = js_now_ms();
        let mut walker_bl = TokenWalker::new(code, mask_bl, language);
        let matches = walker_bl.scan_blacklist_only(blacklist);
        let end_bl = js_now_ms();
        time_ms_with_blacklist = (end_bl - start_bl) as f64;
        blacklist_hits = matches.len() as u32;
    }

    let throughput_line = if time_ms_base > 0.0 { total_lines / time_ms_base } else { 0.0 };
    let throughput_byte = if time_ms_base > 0.0 { total_bytes / time_ms_base } else { 0.0 };

    let mut out = String::new();
    out.push('{');
    push_json_kv_str(&mut out, "time_ms_base", &format!("{:.3}", time_ms_base));
    out.push(',');
    push_json_kv_str(&mut out, "time_ms_with_blacklist", &format!("{:.3}", time_ms_with_blacklist));
    out.push(',');
    push_json_kv_str(&mut out, "throughput_line", &format!("{:.3}", throughput_line));
    out.push(',');
    push_json_kv_str(&mut out, "throughput_byte", &format!("{:.3}", throughput_byte));
    out.push(',');
    push_json_kv_str(&mut out, "symbol_count", &symbol_count.to_string());
    out.push(',');
    push_json_kv_str(&mut out, "blacklist_hits", &blacklist_hits.to_string());
    out.push('}');
    out
}

fn js_now_ms() -> u64 {
    // Fallback timestamp for benchmarking
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
