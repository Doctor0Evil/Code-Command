// FILE: ./core/engine/src/validator.rs

use std::collections::HashSet; // Standard library only, satisfies CC-SOV. [file:2]

/// Represents a single validation request coming from the WASM boundary. [file:2]
pub struct ValidationRequest {
    pub code: String,
    pub tags: Vec<String>,
    /// Optional previous symbol snapshot for CC-CRATE comparisons. [file:2]
    pub previous_symbols: Vec<String>,
}

impl ValidationRequest {
    pub fn from_json(code: &str, tags_json: &str) -> Self {
        // Minimal, hand-rolled JSON parsing for ["TAG", "TAG2"] form to avoid external crates. [file:2]
        let trimmed = tags_json.trim();
        let inner = trimmed
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        let mut tags = Vec::new();
        if !inner.is_empty() {
            for raw in inner.split(',') {
                let t = raw.trim().trim_matches('"');
                if !t.is_empty() {
                    tags.push(t.to_string());
                }
            }
        }

        Self {
            code: code.to_string(),
            tags,
            previous_symbols: Vec::new(),
        }
    }
}

/// Result of running CC- invariant checks over a single artifact. [file:2]
pub struct ValidationResult {
    pub ok: bool,
    pub failures: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            ok: true,
            failures: Vec::new(),
        }
    }

    pub fn fail(&mut self, tag: &str, reason: &str) {
        self.ok = false;
        self.failures
            .push(format!("{}: {}", tag, reason));
    }

    pub fn to_json(&self) -> String {
        // Very small JSON generator: {"ok":true,"failures":["...","..."]} [file:2]
        let mut out = String::new();
        out.push_str("{\"ok\":");
        out.push_str(if self.ok { "true" } else { "false" });
        out.push_str(",\"failures\":[");
        for (i, f) in self.failures.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(f));
            out.push('"');
        }
        out.push_str("]}");
        out
    }
}

/// Runs the full validation pipeline for a request. [file:2]
pub fn run_validation(req: ValidationRequest) -> ValidationResult {
    let mut result = ValidationResult::new();

    let header_path = if req.tags.iter().any(|t| t == "CC-FILE" || t == "CC-DEEP") {
        extract_file_header_path(&req.code)
    } else {
        None
    };

    for tag in &req.tags {
        match tag.as_str() {
            "CC-VOL" => {
                if !check_cc_vol(&req.code, 3) {
                    result.fail("CC-VOL", "Insufficient number of concrete function-like declarations.");
                }
            }
            "CC-LANG" => {
                if let Some(path) = &header_path {
                    if !check_cc_lang(path) {
                        result.fail("CC-LANG", "File extension is not part of the sovereign stack.");
                    }
                }
            }
            "CC-CRATE" => {
                let prev: HashSet<String> = req.previous_symbols.iter().cloned().collect();
                let current = collect_symbols(&req.code);
                if !check_cc_crate(&prev, &current) {
                    result.fail("CC-CRATE", "No new symbols introduced compared to previous snapshot.");
                }
            }
            "CC-FILE" => {
                if header_path.is_none() {
                    result.fail("CC-FILE", "Missing FILE header in first lines of file.");
                }
            }
            "CC-FULL" => {
                if !check_cc_full(&req.code) {
                    result.fail("CC-FULL", "Found excerpt or placeholder markers in code.");
                }
            }
            "CC-DEEP" => {
                if let Some(path) = &header_path {
                    if !check_cc_deep(path) {
                        result.fail("CC-DEEP", "Path does not satisfy depth >= 3 after normalization.");
                    }
                }
            }
            "CC-ZERO" => {
                if !check_cc_zero(&req.code) {
                    result.fail("CC-ZERO", "Entry file contains setup/install/environment references.");
                }
            }
            "CC-PATH" => {
                if let Some(path) = &header_path {
                    if !check_cc_path(path) {
                        result.fail("CC-PATH", "Path contains backslashes, double slashes, or is empty.");
                    }
                }
            }
            "CC-SOV" => {
                if !check_cc_sov(&req.code) {
                    result.fail("CC-SOV", "Detected external crates, tools, or services in imports.");
                }
            }
            "CC-NAV" => {
                if !check_cc_nav(&req.code) {
                    result.fail("CC-NAV", "Custom navigation function not found or external walker detected.");
                }
            }
            _ => {
                // Unknown tag: ignore gracefully for forward-compatibility. [file:2]
            }
        }
    }

    result
}

/* ---------- Tier 1 Checks: Simple String / Byte Scans ---------- */ [file:2]

fn extract_file_header_path(code: &str) -> Option<String> {
    // Look for a FILE header in the first 10 lines: // FILE: ./path, /* FILE: ./path */, <!-- FILE: ./path -->. [file:2]
    for (i, line) in code.lines().enumerate() {
        if i >= 10 {
            break;
        }
        if let Some(idx) = line.find("FILE:") {
            let after = &line[idx + "FILE:".len()..];
            let path = after.trim().trim_start_matches('=').trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn check_cc_vol(code: &str, n_min: usize) -> bool {
    let mut count = 0;
    for line in code.lines() {
        let t = line.trim_start();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("struct ")
            || t.starts_with("impl ")
            || t.starts_with("class ")
        {
            count += 1;
        }
    }
    count >= n_min
}

fn check_cc_lang(path: &str) -> bool {
    // Very small extension checker: path ends-with .rs, .js, .cpp, .h, .aln, .md. [file:2]
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".rs")
        || lower.ends_with(".js")
        || lower.ends_with(".cpp")
        || lower.ends_with(".h")
        || lower.ends_with(".aln")
        || lower.ends_with(".md")
}

fn check_cc_full(code: &str) -> bool {
    let banned = ["...", "rest of code", "omitted"];
    for pat in &banned {
        if code.contains(pat) {
            return false;
        }
    }
    true
}

fn check_cc_deep(path: &str) -> bool {
    let norm = normalize_path(path);
    let parts: Vec<&str> = norm
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();
    parts.len() >= 3
}

fn check_cc_zero(code: &str) -> bool {
    let banned = [
        "install",
        "setup",
        "std::env::temp_dir",
        "std::env::var",
        "pip ",
        "npm ",
        "yarn ",
        "cargo install",
    ];
    for pat in &banned {
        if code.contains(pat) {
            return false;
        }
    }
    true
}

fn check_cc_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.contains('\\') {
        return false;
    }
    if path.contains("//") {
        return false;
    }
    true
}

/* ---------- Tier 2 Checks: Lightweight Token Walker / Parser ---------- */ [file:2]

fn check_cc_crate(previous: &HashSet<String>, current: &HashSet<String>) -> bool {
    for sym in current {
        if !previous.contains(sym) {
            return true;
        }
    }
    false
}

/// Collects a minimal symbol set from code by scanning for fn/struct/impl/mod/class. [file:2]
fn collect_symbols(code: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    for line in code.lines() {
        let t = line.trim_start();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("struct ")
            || t.starts_with("pub struct ")
            || t.starts_with("impl ")
            || t.starts_with("mod ")
            || t.starts_with("pub mod ")
            || t.starts_with("class ")
        {
            let name = extract_identifier_after_keyword(t);
            if !name.is_empty() {
                set.insert(name);
            }
        }
    }
    set
}

fn check_cc_sov(code: &str) -> bool {
    let banned = ["reqwest", "serde_json", "openai", "axios", "tree-sitter", "syn"];
    for line in code.lines() {
        let t = line.trim_start();
        if t.starts_with("use ")
            || t.starts_with("extern crate")
            || t.starts_with("#include")
            || t.starts_with("import ")
            || t.starts_with("require(")
        {
            for pat in &banned {
                if t.contains(pat) {
                    return false;
                }
            }
        }
    }
    true
}

fn check_cc_nav(code: &str) -> bool {
    // Must contain a custom walk_dir that uses read_dir and some recursion pattern. [file:2]
    let has_walk_dir = code.contains("fn walk_dir")
        || code.contains("pub fn walk_dir");
    let mentions_read_dir = code.contains("read_dir(") || code.contains("std::fs::read_dir");
    let banned = ["walkdir::", "globwalk::"];

    if !has_walk_dir || !mentions_read_dir {
        return false;
    }

    for pat in &banned {
        if code.contains(pat) {
            return false;
        }
    }

    true
}

/* ---------- Helper Utilities (No external crates) ---------- */ [file:2]

fn normalize_path(path: &str) -> String {
    // Collapse multiple slashes, remove leading "./". [file:2]
    let mut out = String::new();
    let mut last_was_slash = false;
    for ch in path.chars() {
        if ch == '/' {
            if !last_was_slash {
                out.push(ch);
                last_was_slash = true;
            }
        } else {
            out.push(ch);
            last_was_slash = false;
        }
    }
    if out.starts_with("./") {
        out = out.trim_start_matches("./").to_string();
    }
    out
}

fn extract_identifier_after_keyword(line: &str) -> String {
    // Example: "pub fn foo(bar: i32)" -> "foo". [file:2]
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == '(' || c == '{')
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return String::new();
    }
    for (i, tok) in tokens.iter().enumerate() {
        if *tok == "fn"
            || *tok == "struct"
            || *tok == "impl"
            || *tok == "mod"
            || *tok == "class"
        {
            if let Some(next) = tokens.get(i + 1) {
                return (*next).to_string();
            }
        }
    }
    String::new()
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
