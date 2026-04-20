// FILE: ./core/engine/src/path.rs

use crate::validator::{check_cc_deep, check_cc_path};

pub struct PathCanonicalizer;

impl PathCanonicalizer {
    pub fn new() -> Self {
        PathCanonicalizer
    }

    /// Canonicalize `raw` into a normalized path:
    /// - replace backslashes with forward slashes
    /// - collapse duplicate slashes
    /// - resolve `.` and `..` using a stack
    /// - validate with CC-PATH / CC-DEEP logic.
    /// If Rust normalization fails and the `cpp-fallback` feature is enabled,
    /// call into `cc_sanitize_path` via FFI and re-validate.
    pub fn canonicalize(&self, raw: &str) -> Option<String> {
        if let Some(norm) = self.try_rust_normalize(raw) {
            if self.is_valid_path(&norm) {
                return Some(norm);
            }
        }

        #[cfg(feature = "cpp-fallback")]
        {
            if let Some(norm) = self.try_cpp_fallback(raw) {
                if self.is_valid_path(&norm) {
                    return Some(norm);
                }
            }
        }

        None
    }

    fn try_rust_normalize(&self, raw: &str) -> Option<String> {
        let mut buf = String::with_capacity(raw.len());
        let mut last_was_slash = false;

        for ch in raw.chars() {
            let c = if ch == '\\' { '/' } else { ch };
            if c == '/' {
                if !last_was_slash {
                    buf.push('/');
                    last_was_slash = true;
                }
            } else {
                buf.push(c);
                last_was_slash = false;
            }
        }

        // Remove leading/trailing slashes for segment logic; we re-add leading dot.
        let trimmed = buf.trim_matches('/');
        let mut stack: Vec<String> = Vec::new();

        for part in trimmed.split('/') {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                if stack.pop().is_none() {
                    // attempt to escape root
                    return None;
                }
            } else {
                stack.push(part.to_string());
            }
        }

        let joined = stack.join("/");
        if joined.is_empty() {
            return None;
        }

        // Always return a relative path with leading dot.
        let mut result = String::with_capacity(joined.len() + 2);
        result.push('.');
        result.push('/');
        result.push_str(&joined);
        Some(result)
    }

    #[cfg(feature = "cpp-fallback")]
    fn try_cpp_fallback(&self, raw: &str) -> Option<String> {
        extern "C" {
            fn cc_sanitize_path(ptr: *const u8, len: usize, out_ptr: *mut u8, out_cap: usize)
                -> usize;
        }

        let bytes = raw.as_bytes();
        let mut out = vec![0u8; bytes.len() * 2 + 4];
        let written = unsafe {
            cc_sanitize_path(
                bytes.as_ptr(),
                bytes.len(),
                out.as_mut_ptr(),
                out.len(),
            )
        };

        if written == 0 || written > out.len() {
            return None;
        }

        out.truncate(written);
        String::from_utf8(out).ok()
    }

    fn is_valid_path(&self, norm: &str) -> bool {
        // Reuse existing path/depth validators;
        // this keeps semantics aligned with CC-PATH and CC-DEEP.
        if !check_cc_path(norm) {
            return false;
        }
        if !check_cc_deep(norm) {
            return false;
        }
        true
    }
}
