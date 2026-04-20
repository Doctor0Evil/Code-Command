// FILE: ./core/engine/src/validator.rs

use std::collections::HashSet;
use std::path::Path;

/* ---------- Severity and entry types ---------- */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct ValidationEntry {
    pub tag: String,      // e.g. "CC-FILE"
    pub passed: bool,     // true if this tag's check succeeded
    pub message: String,  // human-readable description
    pub path: String,     // destination from CC-FILE
    pub line: u32,        // 1-based, 0 if not applicable
    pub column: u32,      // 1-based, 0 if not applicable
    pub severity: Severity,
}

/* ---------- Contamination reporting ---------- */

/// Structured report describing a blacklist contamination event.
///
/// This is attached to `ValidationResult` when a (*/)-style blacklist item is
/// found so that callers can inspect precise pattern, location, and severity.
#[derive(Clone, Debug)]
pub struct ContaminationReport {
    pub pattern: String,             // blacklist token, e.g. "Tree-Sitter"
    pub exact_match: String,         // exact substring found in code
    pub surrounding_context: String, // window around match (e.g. line or ±N chars)
    pub severity: String,            // "block" | "warn" | "report"
    pub path: String,                // file path from CC-FILE
    pub line: u32,                   // 1-based line number (best-effort)
    pub column: u32,                 // 1-based column (best-effort)
}

/* ---------- Top-level validation result ---------- */

/// Result of applying all requested CC- checks to a single artifact.
#[derive(Clone, Debug)]
pub struct ValidationResult {
    /// Overall flag: true only if all requested tags passed AND no blocking
    /// blacklist contaminations were observed.
    pub ok: bool,
    /// Per-tag entries including successes and failures.
    pub entries: Vec<ValidationEntry>,
    /// Flattened, human-readable failure messages (for legacy callers).
    pub failures: Vec<String>,
    /// Blacklist contamination events discovered during scanning.
    pub contaminations: Vec<ContaminationReport>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            ok: true,
            entries: Vec::new(),
            failures: Vec::new(),
            contaminations: Vec::new(),
        }
    }

    pub fn record(
        &mut self,
        tag: &str,
        passed: bool,
        message: &str,
        path: &str,
        line: u32,
        column: u32,
        severity: Severity,
    ) {
        if !passed && matches!(severity, Severity::Error) {
            self.ok = false;
        }

        if !passed {
            self.failures
                .push(format!("{}: {}", tag, message));
        }

        self.entries.push(ValidationEntry {
            tag: tag.to_string(),
            passed,
            message: message.to_string(),
            path: path.to_string(),
            line,
            column,
            severity,
        });
    }

    pub fn add_contamination(&mut self, report: ContaminationReport) {
        if report.severity == "block" {
            self.ok = false;
        }
        self.contaminations.push(report);
    }

    /// Serialize into JSON using manual escaping (no external crates).
    ///
    /// Shape:
    /// {
    ///   "ok": true/false,
    ///   "entries": [ ... ],
    ///   "failures": [ "TAG: message", ... ],
    ///   "contaminations": [ ... ]
    /// }
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');

        // ok
        out.push_str("\"ok\":");
        out.push_str(if self.ok { "true" } else { "false" });
        out.push(',');

        // entries
        out.push_str("\"entries\":[");
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"tag\":\"");
            out.push_str(&escape_json(&e.tag));
            out.push_str("\",\"passed\":");
            out.push_str(if e.passed { "true" } else { "false" });
            out.push_str(",\"message\":\"");
            out.push_str(&escape_json(&e.message));
            out.push_str("\",\"path\":\"");
            out.push_str(&escape_json(&e.path));
            out.push_str("\",\"line\":");
            out.push_str(&e.line.to_string());
            out.push_str(",\"column\":");
            out.push_str(&e.column.to_string());
            out.push_str(",\"severity\":\"");
            out.push_str(match e.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            });
            out.push_str("\"}");
        }
        out.push(']');
        out.push(',');

        // failures
        out.push_str("\"failures\":[");
        for (i, f) in self.failures.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(f));
            out.push('"');
        }
        out.push(']');
        out.push(',');

        // contaminations
        out.push_str("\"contaminations\":[");
        for (i, c) in self.contaminations.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"pattern\":\"");
            out.push_str(&escape_json(&c.pattern));
            out.push_str("\",\"exact_match\":\"");
            out.push_str(&escape_json(&c.exact_match));
            out.push_str("\",\"surrounding_context\":\"");
            out.push_str(&escape_json(&c.surrounding_context));
            out.push_str("\",\"severity\":\"");
            out.push_str(&escape_json(&c.severity));
            out.push_str("\",\"path\":\"");
            out.push_str(&escape_json(&c.path));
            out.push_str("\",\"line\":");
            out.push_str(&c.line.to_string());
            out.push_str(",\"column\":");
            out.push_str(&c.column.to_string());
            out.push('}');
        }
        out.push(']');

        out.push('}');
        out
    }
}

/* ---------- Validation request ---------- */

/// A single validation request for a code artifact plus tag set.
pub struct ValidationRequest {
    pub code: String,
    pub tags: Vec<String>,
    /// Optional previous symbol snapshot for CC-CRATE.
    pub previous_symbols: Vec<String>,
}

impl ValidationRequest {
    /// Minimal JSON parser for `["TAG","TAG2"]` to avoid external deps.
    pub fn from_json(code: &str, tags_json: &str) -> Self {
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

/* ---------- Main validation entrypoint ---------- */

/// Main entry for the CC-Engine: run all requested invariant checks.
///
/// This function is pure and deterministic given the same input.
pub fn run_validation(req: ValidationRequest) -> ValidationResult {
    let mut result = ValidationResult::new();

    let need_path = req
        .tags
        .iter()
        .any(|t| t == "CC-FILE" || t == "CC-DEEP" || t == "CC-PATH" || t == "CC-LANG");
    let header_path = if need_path {
        extract_file_header_path(&req.code)
    } else {
        None
    };
    let path_for_entries = header_path
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("<unknown>");

    // Optional blacklist scan.
    if req.tags.iter().any(|t| t == "CC-BLACKLIST") {
        run_blacklist_scan(&req, &header_path, &mut result);
    }

    for tag in &req.tags {
        match tag.as_str() {
            "CC-VOL" => {
                let passed = check_cc_vol(&req.code, 3);
                let msg = if passed {
                    "Volume lock satisfied."
                } else {
                    "Insufficient number of concrete function-like declarations."
                };
                result.record(
                    "CC-VOL",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-LANG" => {
                if let Some(path) = &header_path {
                    let passed = check_cc_lang_path(path);
                    let msg = if passed {
                        "File extension is part of the sovereign stack."
                    } else {
                        "File extension is not part of the sovereign stack (.rs,.js,.cpp,.h,.aln,.md)."
                    };
                    result.record(
                        "CC-LANG",
                        passed,
                        msg,
                        path,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record(
                        "CC-LANG",
                        false,
                        "No FILE header path available for CC-LANG evaluation.",
                        path_for_entries,
                        0,
                        0,
                        Severity::Error,
                    );
                }
            }
            "CC-CRATE" => {
                let prev: HashSet<String> = req.previous_symbols.iter().cloned().collect();
                let current = collect_symbols(&req.code);
                let passed = check_cc_crate(&prev, &current);
                let msg = if passed {
                    "Fresh symbols introduced compared to previous snapshot."
                } else {
                    "No new symbols introduced compared to previous snapshot."
                };
                result.record(
                    "CC-CRATE",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Warning },
                );
            }
            "CC-FILE" => {
                if let Some(path) = &header_path {
                    let passed = !path.trim().is_empty();
                    let msg = if passed {
                        "FILE header found with non-empty path."
                    } else {
                        "FILE header path is empty."
                    };
                    result.record(
                        "CC-FILE",
                        passed,
                        msg,
                        path,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record(
                        "CC-FILE",
                        false,
                        "Missing FILE header in first lines of file.",
                        path_for_entries,
                        0,
                        0,
                        Severity::Error,
                    );
                }
            }
            "CC-FULL" => {
                let passed = check_cc_full(&req.code);
                let msg = if passed {
                    "No excerpt or placeholder markers detected."
                } else {
                    "Found excerpt or placeholder markers in code (\"...\", \"rest of code\", \"omitted\")."
                };
                result.record(
                    "CC-FULL",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-DEEP" => {
                if let Some(path) = &header_path {
                    let passed = check_cc_deep(path);
                    let msg = if passed {
                        "Path satisfies depth >= 3 after normalization."
                    } else {
                        "Path does not satisfy depth >= 3 after normalization."
                    };
                    result.record(
                        "CC-DEEP",
                        passed,
                        msg,
                        path,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record(
                        "CC-DEEP",
                        false,
                        "No FILE header path available for CC-DEEP evaluation.",
                        path_for_entries,
                        0,
                        0,
                        Severity::Error,
                    );
                }
            }
            "CC-ZERO" => {
                let passed = check_cc_zero(&req.code);
                let msg = if passed {
                    "No setup/install/environment references detected."
                } else {
                    "Entry file contains setup/install/environment references."
                };
                result.record(
                    "CC-ZERO",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-PATH" => {
                if let Some(path) = &header_path {
                    let passed = check_cc_path(path);
                    let msg = if passed {
                        "Path passes CC-PATH integrity checks."
                    } else {
                        "Path contains backslashes, double slashes, or is empty."
                    };
                    result.record(
                        "CC-PATH",
                        passed,
                        msg,
                        path,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record(
                        "CC-PATH",
                        false,
                        "No FILE header path available for CC-PATH evaluation.",
                        path_for_entries,
                        0,
                        0,
                        Severity::Error,
                    );
                }
            }
            "CC-SOV" => {
                let passed = check_cc_sov(&req.code);
                let msg = if passed {
                    "No forbidden external crates or tools detected in imports."
                } else {
                    "Detected external crates, tools, or services in imports."
                };
                result.record(
                    "CC-SOV",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-NAV" => {
                let passed = check_cc_nav(&req.code);
                let msg = if passed {
                    "Custom navigation function present and no external walker detected."
                } else {
                    "Custom navigation function not found or external walker detected."
                };
                result.record(
                    "CC-NAV",
                    passed,
                    msg,
                    path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-BLACKLIST" => {
                // Handled above in `run_blacklist_scan`.
            }
            _ => {
                // Unknown tag: ignore for forward-compatibility.
            }
        }
    }

    result
}

/* ---------- Blacklist scanning: contamination reporting ---------- */

fn run_blacklist_scan(
    req: &ValidationRequest,
    header_path: &Option<String>,
    result: &mut ValidationResult,
) {
    // Minimal built-in blacklist for (*/)-style patterns.
    let patterns: &[(&str, &str)] = &[
        ("Rust Syn", "block"),
        ("Tree-Sitter", "block"),
    ];

    let path = header_path
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "<unknown>".to_string());

    for (line_idx, line) in req.code.lines().enumerate() {
        for (pat, severity) in patterns {
            if let Some(col_idx) = line.find(pat) {
                let context = line.trim().to_string();
                let report = ContaminationReport {
                    pattern: pat.to_string(),
                    exact_match: pat.to_string(),
                    surrounding_context: context,
                    severity: severity.to_string(),
                    path: path.clone(),
                    line: (line_idx as u32) + 1,
                    column: (col_idx as u32) + 1,
                };
                result.add_contamination(report);
            }
        }
    }
}

/* ---------- Tier 1: Simple string / byte scans ---------- */

/// Extracts the FILE header path from the first 10 lines if present.
///
/// Accepts headers like:
///   FILE: ./src/core/engine/validator.rs
///   // FILE: ./src/core/engine/validator.rs
///   <!-- FILE: ./src/core/engine/validator.rs -->
fn extract_file_header_path(code: &str) -> Option<String> {
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
            || t.starts_with("pub struct ")
            || t.starts_with("impl ")
            || t.starts_with("mod ")
            || t.starts_with("pub mod ")
            || t.starts_with("class ")
        {
            count += 1;
        }
    }
    count >= n_min
}

/// CC-LANG via extension: .rs, .js, .cpp, .h, .aln, .md only.
fn check_cc_lang_path(path: &str) -> bool {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "rs" | "js" | "cpp" | "h" | "aln" | "md")
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
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty()).collect();
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

/* ---------- Tier 2: Lightweight token walker / parser ---------- */

fn check_cc_crate(previous: &HashSet<String>, current: &HashSet<String>) -> bool {
    for sym in current {
        if !previous.contains(sym) {
            return true;
        }
    }
    false
}

/// Collects a minimal symbol set from code by scanning for fn/struct/impl/mod/class.
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
    let has_walk_dir = code.contains("fn walk_dir") || code.contains("pub fn walk_dir");
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

/* ---------- Helpers (no external crates) ---------- */

fn normalize_path(path: &str) -> String {
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
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == '(' || c == '{')
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return String::new();
    }
    for (i, tok) in tokens.iter().enumerate() {
        if *tok == "fn" || *tok == "struct" || *tok == "impl" || *tok == "mod" || *tok == "class" {
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
