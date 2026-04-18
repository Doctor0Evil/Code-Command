// FILE: ./core/engine/src/navigator.rs

use std::path::{Path, PathBuf}; // Standard library only, satisfies CC-SOV. [file:2]

use crate::vfs::Vfs;

/// Public navigation entry point used by the CC-Engine for native builds.
/// Uses std::fs::read_dir with recursion to satisfy CC-NAV. [file:2]
#[cfg(not(target_arch = "wasm32"))]
pub fn walk_dir(path: &Path) -> Vec<PathBuf> {
    let mut acc = Vec::new();
    walk_dir_recursive(path, &mut acc);
    acc
}

#[cfg(not(target_arch = "wasm32"))]
fn walk_dir_recursive(path: &Path, acc: &mut Vec<PathBuf>) {
    if !path.is_dir() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries {
            if let Ok(e) = entry {
                let p = e.path();
                acc.push(p.clone());
                if p.is_dir() {
                    walk_dir_recursive(&p, acc); // Explicit recursion satisfies CC-NAV. [file:2]
                }
            }
        }
    }
}

/// WASM-targeted navigation using the in-memory VFS abstraction.
/// This mirrors walk_dir semantics using Vfs::list instead of std::fs. [file:2]
#[cfg(target_arch = "wasm32")]
pub fn walk_dir(path: &Path, vfs: &Vfs) -> Vec<String> {
    let base = path.to_string_lossy().to_string();
    let mut acc = Vec::new();
    walk_dir_vfs_recursive(&base, vfs, &mut acc);
    acc
}

#[cfg(target_arch = "wasm32")]
fn walk_dir_vfs_recursive(path: &str, vfs: &Vfs, acc: &mut Vec<String>) {
    acc.push(path.to_string());

    let listing_json = vfs.list(path);
    // listing_json is an array of {"path":"...","is_dir":bool} objects. [file:2]
    let trimmed = listing_json.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return;
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.is_empty() {
        return;
    }

    for raw in inner.split("},{") {
        let mut entry_path = String::new();
        let mut is_dir = false;

        for part in raw.split(',') {
            let p = part.trim();
            if p.starts_with("\"path\"") {
                if let Some(v) = extract_json_value(p) {
                    entry_path = v;
                }
            } else if p.starts_with("\"is_dir\"") {
                if p.contains("true") {
                    is_dir = true;
                }
            }
        }

        if entry_path.is_empty() {
            continue;
        }

        acc.push(entry_path.clone());
        if is_dir {
            walk_dir_vfs_recursive(&entry_path, vfs, acc);
        }
    }
}

/* ---------- Minimal JSON helper, duplicated locally to keep navigator self-contained ---------- */ [file:2]

fn extract_json_value(part: &str) -> Option<String> {
    let mut split = part.splitn(2, ':');
    split.next()?;
    let value_part = split.next()?.trim();
    let value_trimmed = value_part
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_end_matches('}')
        .trim_end_matches(']');
    Some(value_trimmed.to_string())
}
