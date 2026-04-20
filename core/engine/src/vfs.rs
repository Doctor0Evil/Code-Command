// FILE: ./core/engine/src/vfs.rs

use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/* ---------- Sovereign Virtual File System Identity ---------- */

/// Sovereign Virtual File System identity for AI-Chat and CI validators.
pub const CC_VFS_ID: &str = "cc-vfs:1";

/* ---------- Core Types ---------- */

/// A single file entry in the virtual file system, mirroring GitHub metadata.
#[derive(Clone, Debug)]
pub struct FileEntry {
    /// Normalized path (CC-PATH), e.g., "src/main.rs".
    pub path: String,
    /// UTF-8 file content. Empty for directories.
    pub content: String,
    /// SHA or backend-specific version token.
    pub sha: String,
    /// Directory flag.
    pub is_dir: bool,
}

/// The canonical GitHub-backed implementation of the VirtualFileSystem.
///
/// All engine components should talk to `Vfs` through the `VirtualFileSystem`
/// trait to keep the GitHub bridge, browser cache, and other backends
/// interchangeable.
#[derive(Default)]
pub struct Vfs {
    pub files: HashMap<String, FileEntry>,
}

/* ---------- Global VFS for WASM Engine ---------- */

thread_local! {
    /// Global, in-memory VFS instance used by the WASM engine.
    static VFS_INSTANCE: RefCell<Vfs> = RefCell::new(Vfs::default());
}

/* ---------- VirtualFileSystem Trait ---------- */

/// Sovereign virtual filesystem surface for Code-Command.
///
/// This trait is the canonical cc-vfs API. All engine components and CC-API
/// exports should depend on this contract rather than on concrete details of
/// the GitHub bridge, browser cache, or storage backend.
pub trait VirtualFileSystem {
    /// Read the textual content of a file at `path`.
    ///
    /// Invariants:
    /// - `path` MUST be normalized via `normalize_path` before use.
    /// - Returns `None` if the entry is a directory or does not exist.
    fn read(&self, path: &str) -> Option<String>;

    /// Write `content` to `path` with the given `sha`, creating or replacing
    /// an entry and returning `true` on success.
    ///
    /// Invariants:
    /// - MUST enforce `CC-PATH` via `normalize_path(path)` and reject malformed
    ///   or empty paths.
    /// - MUST enforce `CC-DEEP` for core modules: writing to key engine paths
    ///   requires depth ≥ 3.
    /// - MUST keep `is_dir == false` for written entries.
    /// - MUST update in-memory state before notifying any external backend.
    fn write(&mut self, path: &str, content: &str, sha: &str) -> bool;

    /// List direct children under `path` as a JSON array string.
    ///
    /// Format:
    /// [
    ///   {"path":"a/b","isdir":true,"sha":""},
    ///   {"path":"a/b/c.rs","isdir":false,"sha":"..."}
    /// ]
    ///
    /// Invariants:
    /// - MUST include only direct children (no grandchildren).
    /// - MUST emit normalized paths.
    fn list(&self, path: &str) -> String;

    /// Construct a VFS instance from a JSON snapshot.
    ///
    /// Expected format:
    /// [
    ///   {"path":"...","content":"...","sha":"...","isdir":true|false},
    ///   ...
    /// ]
    ///
    /// Invariants:
    /// - MUST call `normalize_path` on each `path` before insertion.
    /// - MUST treat missing `content` as empty string for directories.
    /// - MUST ignore malformed entries rather than panic.
    fn from_json(json: &str) -> Self
    where
        Self: Sized;

    /// Serialize the entire VFS state into a JSON snapshot string.
    ///
    /// Format is intentionally symmetric with `from_json` so that:
    /// - `Self::from_json(v.to_json())` reconstructs an equivalent state
    ///   (up to key ordering and normalization).
    ///
    /// Invariants:
    /// - MUST emit valid JSON with proper string escaping.
    /// - MUST emit all entries currently present in memory.
    /// - MUST only emit normalized paths.
    fn to_json(&self) -> String;
}

/* ---------- VirtualFileSystem Implementation for Vfs ---------- */

impl VirtualFileSystem for Vfs {
    fn read(&self, path: &str) -> Option<String> {
        let norm = normalize_path(path);
        if norm.is_empty() {
            return None;
        }

        if let Some(entry) = self.files.get(&norm) {
            if !entry.is_dir {
                return Some(entry.content.clone());
            }
        }

        // Fallback: ask JS layer to fetch the file via GitHub Contents API.
        if let Ok(encoded) = js_fetch_file(&norm) {
            let decoded = base64_decode(&encoded);
            return Some(decoded);
        }

        None
    }

    fn write(&mut self, path: &str, content: &str, sha: &str) -> bool {
        let norm = normalize_path(path);
        if norm.is_empty() {
            return false;
        }

        // Enforce CC-DEEP / CC-PATH for core modules and protected paths.
        if !path_depth_ok(&norm) {
            return false;
        }

        // Update local map first (so in-memory state is always authoritative).
        let entry = FileEntry {
            path: norm.clone(),
            content: content.to_string(),
            sha: sha.to_string(),
            is_dir: false,
        };
        self.files.insert(norm.clone(), entry);

        // Notify JS so it can perform the GitHub write/commit.
        let encoded = base64_encode(content);
        js_write_file(&norm, &encoded, sha)
    }

    fn list(&self, path: &str) -> String {
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
            out.push('{');
            out.push_str("\"path\":\"");
            out.push_str(&escape_json(&e.path));
            out.push_str("\",\"isdir\":");
            out.push_str(if e.is_dir { "true" } else { "false" });
            out.push_str(",\"sha\":\"");
            out.push_str(&escape_json(&e.sha));
            out.push('"');
            out.push('}');
        }

        out.push(']');
        out
    }

    fn from_json(json: &str) -> Self {
        let mut files: HashMap<String, FileEntry> = HashMap::new();
        let trimmed = json.trim();

        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Self { files };
        }

        let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
        if inner.trim().is_empty() {
            return Self { files };
        }

        // Very simple split on "},{" boundaries; assumes well-formed producer.
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
                } else if p.starts_with("\"isdir\"") || p.starts_with("\"is_dir\"") {
                    if p.contains("true") {
                        is_dir = true;
                    }
                }
            }

            if !path.is_empty() {
                let norm = normalize_path(&path);
                if norm.is_empty() {
                    continue;
                }
                let entry = FileEntry {
                    path: norm.clone(),
                    content: if is_dir { String::new() } else { content },
                    sha,
                    is_dir,
                };
                files.insert(norm, entry);
            }
        }

        Self { files }
    }

    fn to_json(&self) -> String {
        // Snapshot format mirrors from_json expectations:
        // [
        //   {"path":"...","content":"...","sha":"...","isdir":true|false},
        //   ...
        // ]
        let mut out = String::new();
        out.push('[');

        let mut first = true;
        for entry in self.files.values() {
            if !first {
                out.push(',');
            }
            first = false;

            out.push('{');

            // "path": "<normalized path>"
            out.push_str("\"path\":\"");
            out.push_str(&escape_json(&entry.path));
            out.push_str("\",");

            // "content": "<file content or empty for directories>"
            out.push_str("\"content\":\"");
            if !entry.is_dir {
                out.push_str(&escape_json(&entry.content));
            }
            out.push_str("\",");

            // "sha": "<sha string>"
            out.push_str("\"sha\":\"");
            out.push_str(&escape_json(&entry.sha));
            out.push_str("\",");

            // "isdir": true|false
            out.push_str("\"isdir\":");
            out.push_str(if entry.is_dir { "true" } else { "false" });

            out.push('}');
        }

        out.push(']');
        out
    }
}

/* ---------- WASM Snapshot Export (Uint8Array-friendly) ---------- */

/// WASM export returning a pointer + length into linear memory.
///
/// JS should:
/// - call `cc_vfs_snapshot_ptr(&mut len)`
/// - copy `len` bytes from `ptr` into a fresh `Uint8Array`
/// - decode as UTF-8 JSON using TextDecoder
#[no_mangle]
pub extern "C" fn cc_vfs_snapshot_ptr(len_out: *mut u32) -> *const u8 {
    thread_local! {
        static SNAPSHOT_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    }

    SNAPSHOT_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        let json = VFS_INSTANCE.with(|v| v.borrow().to_json());
        buf.extend_from_slice(json.as_bytes());
        unsafe {
            *len_out = buf.len() as u32;
        }
        buf.as_ptr()
    })
}

/* ---------- JS Bridge: GitHub API Shims ---------- */

#[wasm_bindgen(module = "/js/app/github/api.js")]
extern "C" {
    /// Fetches a file's Base64 content from GitHub via JS.
    #[wasm_bindgen(js_name = "wasmFetchFileBase64")]
    fn js_fetch_file(path: &str) -> Result<String, JsValue>;

    /// Writes a Base64-encoded file back to GitHub via JS; returns success flag.
    #[wasm_bindgen(js_name = "wasmWriteFileBase64")]
    fn js_write_file(path: &str, content_base64: &str, sha: &str) -> bool;
}

/* ---------- Path & Depth Helpers (CC-PATH, CC-DEEP) ---------- */

/// Normalize a path according to CC-PATH invariants:
/// - Trim leading/trailing whitespace
/// - Reject control characters (ASCII 0-31)
/// - Replace backslashes with forward slashes
/// - Collapse multiple slashes into one
/// - Remove leading "./"
/// - Reject paths containing ".." segments that would escape repo root
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();

    // Reject control characters (ASCII 0-31).
    for ch in trimmed.chars() {
        if (ch as u32) < 32 {
            return String::new();
        }
    }

    let mut out = String::new();
    let mut last_was_slash = false;

    for ch in trimmed.chars() {
        if ch == '\\' {
            // Replace Windows-style separators with forward slashes.
            if !last_was_slash {
                out.push('/');
                last_was_slash = true;
            }
        } else if ch == '/' {
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
    while norm.starts_with("./") {
        norm = norm[2..].to_string();
    }

    // Reject if path contains ".." segments that could escape root.
    if norm.starts_with("..") || norm.contains("/../") {
        return String::new();
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
        // Root listing: entries without further slashes.
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

    // Direct child if tail has no additional '/'.
    !tail.contains('/')
}

/* ---------- Minimal JSON and Base64 Utilities ---------- */

fn extract_json_value(part: &str) -> Option<String> {
    // Expects something like: "\"key\":\"value\"" or "\"key\":\"value\"}"
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

// Minimal Base64, aligned with GitHub API requirements, using a custom alphabet.
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
    let mut count = 0;

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

    // Handle remaining 2 or 3 bytes if any.
    if count > 1 {
        let mut n = (chunk[0] as u32) << 18;
        n |= (chunk[1] as u32) << 12;
        if count == 3 {
            n |= (chunk[2] as u32) << 6;
        }
        buf.push(((n >> 16) & 0xFF) as u8);
        if count == 3 {
            buf.push(((n >> 8) & 0xFF) as u8);
        }
    }

    String::from_utf8_lossy(&buf).into_owned()
}

fn b64_index(ch: char) -> Option<u8> {
    for (i, &c) in B64_ALPHABET.iter().enumerate() {
        if c as char == ch {
            return Some(i as u8);
        }
    }
    None
}

/* ---------- Unit Tests for Path Normalization & Utilities ---------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic_path() {
        assert_eq!(normalize_path("src/main.rs"), "src/main.rs");
        assert_eq!(normalize_path("./src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_normalize_whitespace_trimming() {
        assert_eq!(normalize_path("  src/main.rs  "), "src/main.rs");
        // Control char should cause rejection.
        let s = format!("src{}main.rs", '\u{0000}');
        assert_eq!(normalize_path(&s), "");
    }

    #[test]
    fn test_normalize_backslash_conversion() {
        assert_eq!(normalize_path("src\\main.rs"), "src/main.rs");
        assert_eq!(normalize_path("src\\\\main.rs"), "src/main.rs");
        assert_eq!(normalize_path("src\\sub\\file.rs"), "src/sub/file.rs");
    }

    #[test]
    fn test_normalize_collapse_slashes() {
        assert_eq!(normalize_path("src//main.rs"), "src/main.rs");
        assert_eq!(normalize_path("src///main.rs"), "src/main.rs");
        assert_eq!(normalize_path("//src/main.rs"), "/src/main.rs");
    }

    #[test]
    fn test_normalize_reject_parent_escape() {
        assert_eq!(normalize_path("../etc/passwd"), "");
        assert_eq!(normalize_path("../../etc/passwd"), "");
        assert_eq!(normalize_path("/../etc/passwd"), "");
        assert_eq!(normalize_path("src/../main.rs"), "src/../main.rs");
    }

    #[test]
    fn test_path_depth_ok_basic() {
        assert!(path_depth_ok("src/lib/main.rs"));
        assert!(path_depth_ok("a/b/c/d.rs"));
        assert!(!path_depth_ok("src/main.rs"));
        assert!(!path_depth_ok("main.rs"));
    }

    #[test]
    fn test_is_direct_child_of() {
        assert!(is_direct_child_of("src/main.rs", "src"));
        assert!(is_direct_child_of("src.rs", ""));
        assert!(!is_direct_child_of("src/sub/main.rs", "src"));
        assert!(!is_direct_child_of("other/main.rs", "src"));
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = "Hello, 世界! This is a test string for base64.";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_escape_json() {
        let input = "Line1\nLine2\r\n\tTabbed\"Quote\\Backslash";
        let escaped = escape_json(input);
        assert_eq!(
            escaped,
            "Line1\\nLine2\\r\\n\\tTabbed\\\"Quote\\\\Backslash"
        );
    }
}
