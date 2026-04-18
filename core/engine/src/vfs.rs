// FILE: ./core/engine/src/vfs.rs

use std::collections::HashMap; // Standard library only, satisfies CC-SOV. [file:2]

use wasm_bindgen::prelude::*; // Build-time JS glue only; no external runtime crates. [file:2]

/// A single file entry in the virtual file system, mirroring GitHub metadata. [file:2]
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub content: String,
    pub sha: String,
    pub is_dir: bool,
}

/// Virtual File System used by the CC-Engine.
/// Backed by JS shims that talk to the GitHub API. [file:2]
#[derive(Default)]
pub struct Vfs {
    files: HashMap<String, FileEntry>,
}

impl Vfs {
    /// Create a VFS instance from a JSON snapshot provided by JS. [file:2]
    pub fn from_json(json: &str) -> Self {
        // Minimal parser for an array of {path, content, sha, is_dir}. [file:2]
        let mut files = HashMap::new();
        let trimmed = json.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Self { files };
        }

        // Very simple split on "},{" boundaries; assumes well-formed producer. [file:2]
        let inner = &trimmed[1..trimmed.len() - 1];
        for raw in inner.split("},{") {
            let mut path = String::new();
            let mut content = String::new();
            let mut sha = String::new();
            let mut is_dir = false;

            for part in raw.split(',') {
                let p = part.trim();
                if p.starts_with("\"path\"") {
                    if let Some(v) = extract_json_value(p) {
                        path = v;
                    }
                } else if p.starts_with("\"content\"") {
                    if let Some(v) = extract_json_value(p) {
                        content = v;
                    }
                } else if p.starts_with("\"sha\"") {
                    if let Some(v) = extract_json_value(p) {
                        sha = v;
                    }
                } else if p.starts_with("\"is_dir\"") {
                    if p.contains("true") {
                        is_dir = true;
                    }
                }
            }

            if !path.is_empty() {
                let norm = normalize_path(&path);
                let entry = FileEntry {
                    path: norm.clone(),
                    content,
                    sha,
                    is_dir,
                };
                files.insert(norm, entry);
            }
        }

        Self { files }
    }

    /// Reads a file from the VFS, fetching from JS/GitHub if needed. [file:2]
    pub fn read(&self, path: &str) -> Option<String> {
        let norm = normalize_path(path);
        if let Some(entry) = self.files.get(&norm) {
            if !entry.is_dir {
                return Some(entry.content.clone());
            }
        }

        // Fallback: ask JS layer to fetch the file via GitHub Contents API. [file:2]
        if let Ok(encoded) = js_fetch_file(&norm) {
            let decoded = base64_decode(&encoded);
            return Some(decoded);
        }

        None
    }

    /// Writes a file into the VFS and notifies JS/GitHub via Contents API semantics. [file:2]
    pub fn write(&mut self, path: &str, content: &str, sha: &str) -> bool {
        let norm = normalize_path(path);

        // Enforce CC-DEEP for core modules: require depth >= 3. [file:2]
        if !path_depth_ok(&norm) {
            return false;
        }

        // Update local map. [file:2]
        let entry = FileEntry {
            path: norm.clone(),
            content: content.to_string(),
            sha: sha.to_string(),
            is_dir: false,
        };
        self.files.insert(norm.clone(), entry);

        // Notify JS so it can perform the GitHub write/commit. [file:2]
        let encoded = base64_encode(content);
        js_write_file(&norm, &encoded, sha)
    }

    /// Lists entries under a directory path, returning a JSON array string. [file:2]
    pub fn list(&self, path: &str) -> String {
        let norm = normalize_path(path);
        let mut entries = Vec::new();

        for (p, entry) in &self.files {
            if is_direct_child_of(p, &norm) {
                entries.push(entry);
            }
        }

        let mut out = String::new();
        out.push('[');
        for (i, e) in entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"path\":\"");
            out.push_str(&escape_json(&e.path));
            out.push_str("\",\"is_dir\":");
            out.push_str(if e.is_dir { "true" } else { "false" });
            out.push_str(",\"sha\":\"");
            out.push_str(&escape_json(&e.sha));
            out.push_str("\"}");
        }
        out.push(']');
        out
    }
}

/* ---------- JS Bridge: GitHub API Shims ---------- */ [file:2]

#[wasm_bindgen(module = "/js/app/github/api.js")]
extern "C" {
    /// Fetches a file's Base64 content from GitHub via JS. [file:2]
    #[wasm_bindgen(js_name = "wasmFetchFileBase64")]
    fn js_fetch_file(path: &str) -> Result<String, JsValue>;

    /// Writes a Base64-encoded file back to GitHub via JS; returns success flag. [file:2]
    #[wasm_bindgen(js_name = "wasmWriteFileBase64")]
    fn js_write_file(path: &str, content_base64: &str, sha: &str) -> bool;
}

/* ---------- Path & Depth Helpers (CC-PATH, CC-DEEP) ---------- */ [file:2]

fn normalize_path(path: &str) -> String {
    let mut out = String::new();
    let mut last_was_slash = false;

    for ch in path.chars() {
        if ch == '\\' {
            // CC-PATH: disallow Windows-style separators. [file:2]
            continue;
        }
        if ch == '/' {
            if !last_was_slash {
                out.push('/');
                last_was_slash = true;
            }
        } else {
            out.push(ch);
            last_was_slash = false;
        }
    }

    let mut norm = out;
    if norm.starts_with("./") {
        norm = norm.trim_start_matches("./").to_string();
    }
    norm
}

fn path_depth_ok(path: &str) -> bool {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();
    parts.len() >= 3
}

fn is_direct_child_of(candidate: &str, parent: &str) -> bool {
    let parent_norm = normalize_path(parent);
    let c_norm = normalize_path(candidate);

    if parent_norm.is_empty() {
        // Root listing: entries without further slashes. [file:2]
        return !c_norm.contains('/');
    }

    if !c_norm.starts_with(&parent_norm) {
        return false;
    }

    let rest = &c_norm[parent_norm.len()..];
    if !rest.starts_with('/') {
        return false;
    }
    let tail = &rest[1..];

    // Direct child if tail has no additional '/'. [file:2]
    !tail.contains('/')
}

/* ---------- Minimal JSON and Base64 Utilities ---------- */ [file:2]

fn extract_json_value(part: &str) -> Option<String> {
    // Expects something like: "key":"value" or "key":"value"} [file:2]
    let mut split = part.splitn(2, ':');
    split.next()?; // key
    let value_part = split.next()?.trim();
    let value_trimmed = value_part
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_end_matches('}')
        .trim_end_matches(']');
    Some(value_trimmed.to_string())
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

// Minimal Base64, aligned with GitHub API requirements, using a custom alphabet. [file:2]
const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;

    while i + 3 <= bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        let b2 = bytes[i + 2] as u32;

        let n = (b0 << 16) | (b1 << 8) | b2;

        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(B64_ALPHABET[(n & 63) as usize] as char);

        i += 3;
    }

    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i] as u32;
        let n = b0 << 16;

        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        let n = (b0 << 16) | (b1 << 8);

        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }

    out
}

fn base64_decode(input: &str) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4];
    let mut count = 0usize;

    for ch in input.chars() {
        if ch == '=' {
            break;
        }
        if let Some(idx) = b64_index(ch) {
            chunk[count] = idx;
            count += 1;
            if count == 4 {
                let n = ((chunk[0] as u32) << 18)
                    | ((chunk[1] as u32) << 12)
                    | ((chunk[2] as u32) << 6)
                    | (chunk[3] as u32);
                buf.push(((n >> 16) & 0xFF) as u8);
                buf.push(((n >> 8) & 0xFF) as u8);
                buf.push((n & 0xFF) as u8);
                count = 0;
            }
        }
    }

    // Handle padding cases based on original input length. [file:2]
    let padding = input.chars().rev().take_while(|c| *c == '=').count();
    if padding > 0 && !buf.is_empty() {
        buf.truncate(buf.len().saturating_sub(padding));
    }

    String::from_utf8_lossy(&buf).into_owned()
}

fn b64_index(ch: char) -> Option<u8> {
    for (i, c) in B64_ALPHABET.iter().enumerate() {
        if *c as char == ch {
            return Some(i as u8);
        }
    }
    None
}
