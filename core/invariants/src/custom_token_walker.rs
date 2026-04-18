// FILE: ./core/invariants/src/custom_token_walker.rs
// This file uses ONLY Rust Standard Library (std). 
// NO EXTERNAL CRATES (violates R9).
// This is a custom AST-like scanner for Code-Command tags.

use std::path::Path;

/// Custom Token Walker for Code-Command Invariants
/// This is a hand-written state machine to avoid 'syn' or 'regex' crates.
pub struct CCTokenWalker {
    content: String,
}

impl CCTokenWalker {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    /// CC-PATH Invariant: Check for malformed paths (double slashes or backslashes)
    pub fn check_path_integrity(&self) -> bool {
        // Custom scan without external regex library
        let bytes = self.content.as_bytes();
        for i in 0..bytes.len().saturating_sub(1) {
            if bytes[i] == b'/' && bytes[i + 1] == b'/' {
                return false; // Found double slash "//" in a path string context
            }
            if bytes[i] == b'\\' {
                return false; // Windows-style path detected, violates portability
            }
        }
        true
    }

    /// CC-FILE Invariant: Ensure a file destination is declared in first 10 lines
    pub fn check_file_destination(&self) -> bool {
        // Custom scan for "// FILE: "
        let pattern = "// FILE: ";
        let lines = self.content.lines().take(10);
        for line in lines {
            if line.starts_with(pattern) {
                // Validate the path depth (CC-DEEP logic could go here)
                let path_part = &line[pattern.len()..].trim();
                let path = Path::new(path_part);
                // Check depth >= 3
                if path.components().count() >= 3 {
                    return true;
                }
            }
        }
        false
    }

    /// CC-FULL Invariant: No placeholders like "..."
    pub fn check_no_excerpts(&self) -> bool {
        !self.content.contains("...") && 
        !self.content.contains("// rest of code") &&
        !self.content.contains("/* omitted */")
    }
}
