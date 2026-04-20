// FILE: ./core/engine/src/validator.rs
// FILE .core/engine/src/validator.rs
//
// Tiered invariant validator for Code-Command.
//
// Implements VALIDATOR-CONTRACT-1 from .specs/engine.aln and enforces
// CC- tags using a combination of Tier 1 string/line scans and
// Tier 2 structural recognition via the token walker.
//
// Invariants:
// - CC-LANG: Rust only, no external crates beyond std.
// - CC-SOV: No use of syn, regex, tree-sitter, etc.
// - CC-FULL: No "omitted" code sections; this file is complete.
// - CC-PATH: No malformed paths in FILE header logic.
// - CC-NAV: Uses only custom navigation checks via tokenwalker.
// - CC-VOL: Ensures sufficient function/struct declarations per file.
// - CC-CRATE: Tracks new symbols relative to previous snapshot.

use std::collections::HashSet;

mod tokenwalker;

use tokenwalker::{
    LanguageHint,
    ScanProfile,
    ScanProfileId,
    TokenWalker,
};

/* ---------- Core contract structs (VALIDATOR-CONTRACT-1) ---------- */

/// Location within a file for validation failures.
#[derive(Clone, Debug)]
pub struct FailureLocation {
    pub line: u32,
    pub column: u32,
}

/// ValidationFailure as defined in VALIDATOR-CONTRACT-1.
///
/// {
///   "tag": string,
///   "message": string,
///   "location"?: { "line": integer, "column": integer }
/// }
#[derive(Clone, Debug)]
pub struct ValidationFailure {
    pub tag: String,
    pub message: String,
    pub location: Option<FailureLocation>,
}

/// ValidationRequest as defined in VALIDATOR-CONTRACT-1.
///
/// {
///   "code": string,
///   "path": string,
///   "tags": [string],
///   "previous_symbols"?: [string]
/// }
#[derive(Clone, Debug)]
pub struct ValidationRequest {
    pub code: String,
    pub path: String,
    pub tags: Vec<String>,
    pub previous_symbols: Option<Vec<String>>,
}

impl ValidationRequest {
    /// Construct a ValidationRequest from raw inputs.
    /// tags_json is a JSON array of strings, e.g. ["CC-FILE","CC-PATH"].
    pub fn from_raw(code: &str, path: &str, tags_json: &str) -> Self {
        let tags = parse_tags_json(tags_json);
        ValidationRequest {
            code: code.to_string(),
            path: path.to_string(),
            tags,
            previous_symbols: None,
        }
    }
}

/* ---------- Severity, contamination, and extended reporting ---------- */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Structured report describing a blacklist contamination event.
///
/// This is attached to `ValidationResult` when a (*/)-style blacklist item is
/// found so that callers can inspect precise pattern, location, and severity.
#[derive(Clone, Debug)]
pub struct ContaminationReport {
    pub pattern: String,             // blacklist token, e.g. "Tree-Sitter"
    pub exact_match: String,         // exact substring found in code
    pub surrounding_context: String, // window around match (e.g. line text)
    pub severity: String,            // "block" | "warn" | "report"
    pub path: String,                // file path from CC-FILE or ValidationRequest.path
    pub line: u32,                   // 1-based line number (best-effort)
    pub column: u32,                 // 1-based column (best-effort)
}

/// Per-tag entry, including successes and failures.
#[derive(Clone, Debug)]
pub struct ValidationEntry {
    pub tag: String,      // e.g. "CC-FILE"
    pub passed: bool,     // true if this tag's check succeeded
    pub message: String,  // human-readable description
    pub path: String,     // destination from CC-FILE or request.path
    pub line: u32,        // 1-based, 0 if not applicable
    pub column: u32,      // 1-based, 0 if not applicable
    pub severity: Severity,
}

/// ValidationResult as defined in VALIDATOR-CONTRACT-1 plus richer detail.
///
/// CONTRACT FIELDS:
/// {
///   "ok": boolean,
///   "error_code"?: string,
///   "failures": [ValidationFailure],
///   "new_symbols"?: [string]
/// }
///
/// EXTENSIONS (not part of the minimal contract wire format but reflected
/// in to_json for richer diagnostics):
/// - entries: per-tag summary
/// - contaminations: blacklist events
#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub ok: bool,
    pub error_code: Option<String>,
    pub failures: Vec<ValidationFailure>,
    pub new_symbols: Option<Vec<String>>,

    // Extended reporting
    pub entries: Vec<ValidationEntry>,
    pub contaminations: Vec<ContaminationReport>,
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            ok: true,
            error_code: None,
            failures: Vec::new(),
            new_symbols: None,
            entries: Vec::new(),
            contaminations: Vec::new(),
        }
    }

    /// Contract-level failure recording (VALIDATOR-CONTRACT-1).
    pub fn fail(&mut self, tag: &str, message: &str, location: Option<FailureLocation>) {
        self.ok = false;
        if self.error_code.is_none() {
            // VALIDATION-FAILED error code per engine.aln
            self.error_code = Some("CCENG-020".to_string());
        }
        self.failures.push(ValidationFailure {
            tag: tag.to_string(),
            message: message.to_string(),
            location: location.clone(),
        });

        // Also mirror into entries with Severity::Error.
        let (line, column) = location
            .map(|loc| (loc.line, loc.column))
            .unwrap_or((0, 0));
        self.entries.push(ValidationEntry {
            tag: tag.to_string(),
            passed: false,
            message: message.to_string(),
            path: String::new(),
            line,
            column,
            severity: Severity::Error,
        });
    }

    pub fn record_entry(
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
            if self.error_code.is_none() {
                self.error_code = Some("CCENG-020".to_string());
            }
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

        if !passed {
            self.failures.push(ValidationFailure {
                tag: tag.to_string(),
                message: message.to_string(),
                location: if line > 0 || column > 0 {
                    Some(FailureLocation { line, column })
                } else {
                    None
                },
            });
        }
    }

    pub fn add_contamination(&mut self, report: ContaminationReport) {
        if report.severity == "block" {
            self.ok = false;
            if self.error_code.is_none() {
                self.error_code = Some("CCENG-020".to_string());
            }
        }
        self.contaminations.push(report);
    }

    /// Serialize into JSON using manual escaping (no external crates).
    ///
    /// Shape:
    /// {
    ///   "ok": true/false,
    ///   "error_code"?: "CCENG-020",
    ///   "failures": [ ... ],
    ///   "new_symbols"?: [ ... ],
    ///   "entries": [ ... ],
    ///   "contaminations": [ ... ]
    /// }
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');

        // ok
        out.push_str("\"ok\":");
        out.push_str(if self.ok { "true" } else { "false" });

        // error_code
        if let Some(ref code) = self.error_code {
            out.push_str(",\"error_code\":\"");
            out.push_str(&escape_json_string(code));
            out.push('"');
        }

        // failures
        out.push_str(",\"failures\":[");
        for (i, f) in self.failures.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"tag\":\"");
            out.push_str(&escape_json_string(&f.tag));
            out.push('"');
            out.push_str(",\"message\":\"");
            out.push_str(&escape_json_string(&f.message));
            out.push('"');
            if let Some(ref loc) = f.location {
                out.push_str(",\"location\":{");
                out.push_str("\"line\":");
                out.push_str(&loc.line.to_string());
                out.push_str(",\"column\":");
                out.push_str(&loc.column.to_string());
                out.push('}');
            }
            out.push('}');
        }
        out.push(']');

        // new_symbols
        if let Some(ref syms) = self.new_symbols {
            out.push_str(",\"new_symbols\":[");
            for (i, s) in syms.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(&escape_json_string(s));
                out.push('"');
            }
            out.push(']');
        }

        // entries (extended)
        out.push_str(",\"entries\":[");
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"tag\":\"");
            out.push_str(&escape_json_string(&e.tag));
            out.push('"');
            out.push_str(",\"passed\":");
            out.push_str(if e.passed { "true" } else { "false" });
            out.push_str(",\"message\":\"");
            out.push_str(&escape_json_string(&e.message));
            out.push('"');
            out.push_str(",\"path\":\"");
            out.push_str(&escape_json_string(&e.path));
            out.push('"');
            out.push_str(",\"line\":");
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

        // contaminations (extended)
        out.push_str(",\"contaminations\":[");
        for (i, c) in self.contaminations.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"pattern\":\"");
            out.push_str(&escape_json_string(&c.pattern));
            out.push('"');
            out.push_str(",\"exact_match\":\"");
            out.push_str(&escape_json_string(&c.exact_match));
            out.push('"');
            out.push_str(",\"surrounding_context\":\"");
            out.push_str(&escape_json_string(&c.surrounding_context));
            out.push('"');
            out.push_str(",\"severity\":\"");
            out.push_str(&escape_json_string(&c.severity));
            out.push('"');
            out.push_str(",\"path\":\"");
            out.push_str(&escape_json_string(&c.path));
            out.push('"');
            out.push_str(",\"line\":");
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

/* ---------- Public entrypoint used by lib.rs ---------- */

/// Main entry for the CC-Engine: run all requested invariant checks.
///
/// This function is pure and deterministic given the same input.
pub fn run_validation(req: &ValidationRequest) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Extract FILE header path for path-related checks.
    let header_path = extract_file_header_path(&req.code);
    let fallback_path = if req.path.is_empty() {
        "<unknown>".to_string()
    } else {
        req.path.clone()
    };
    let path_for_entries = header_path
        .as_ref()
        .cloned()
        .unwrap_or_else(|| fallback_path.clone());

    // Build a ScanProfile for Tier 2 checks based on requested tags.
    let profile = build_scan_profile(&req.tags);
    let walker = TokenWalker::new(&req.code, LanguageHint::Rust);
    let scan = walker.scan(&profile);

    // Precompute symbol sets for CC-CRATE and CC-VOL.
    let symbol_set: HashSet<String> = scan
        .symbols
        .iter()
        .cloned()
        .collect();

    // Optional blacklist scan (CC-BLACKLIST).
    if req.tags.iter().any(|t| t == "CC-BLACKLIST") {
        run_blacklist_scan(&req, &header_path, &mut result);
    }

    // Apply checks in a deterministic order.
    for tag in &req.tags {
        match tag.as_str() {
            "CC-FILE" => {
                if let Some(ref hp) = header_path {
                    let passed = !hp.trim().is_empty();
                    let msg = if passed {
                        "FILE header found with non-empty path."
                    } else {
                        "FILE header path is empty."
                    };
                    result.record_entry(
                        "CC-FILE",
                        passed,
                        msg,
                        hp,
                        1,
                        1,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record_entry(
                        "CC-FILE",
                        false,
                        "Missing FILE header in first lines of file.",
                        &path_for_entries,
                        1,
                        1,
                        Severity::Error,
                    );
                }
            }
            "CC-LANG" => {
                if let Some(ref hp) = header_path {
                    let passed = check_cc_lang(hp);
                    let msg = if passed {
                        "File extension is part of the sovereign stack."
                    } else {
                        "File extension is not part of the sovereign stack (.rs,.js,.cpp,.h,.aln,.md)."
                    };
                    result.record_entry(
                        "CC-LANG",
                        passed,
                        msg,
                        hp,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record_entry(
                        "CC-LANG",
                        false,
                        "Cannot determine language without FILE header path.",
                        &path_for_entries,
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
                result.record_entry(
                    "CC-FULL",
                    passed,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-PATH" => {
                if let Some(ref hp) = header_path {
                    let passed = check_cc_path(hp);
                    let msg = if passed {
                        "Path passes CC-PATH integrity checks."
                    } else {
                        "Path contains backslashes, double slashes, or is empty."
                    };
                    result.record_entry(
                        "CC-PATH",
                        passed,
                        msg,
                        hp,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record_entry(
                        "CC-PATH",
                        false,
                        "Missing FILE header path for CC-PATH check.",
                        &path_for_entries,
                        0,
                        0,
                        Severity::Error,
                    );
                }
            }
            "CC-DEEP" => {
                if let Some(ref hp) = header_path {
                    let passed = check_cc_deep(hp);
                    let msg = if passed {
                        "Path satisfies depth >= 3 after normalization."
                    } else {
                        "Path does not satisfy depth >= 3 after normalization."
                    };
                    result.record_entry(
                        "CC-DEEP",
                        passed,
                        msg,
                        hp,
                        0,
                        0,
                        if passed { Severity::Info } else { Severity::Error },
                    );
                } else {
                    result.record_entry(
                        "CC-DEEP",
                        false,
                        "Missing FILE header path for CC-DEEP check.",
                        &path_for_entries,
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
                result.record_entry(
                    "CC-ZERO",
                    passed,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-SOV" => {
                let passed = check_cc_sov(&scan.import_lines);
                let msg = if passed {
                    "No forbidden external crates or tools detected in imports."
                } else {
                    "Detected external crates, tools, or services in imports."
                };
                result.record_entry(
                    "CC-SOV",
                    passed,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-VOL" => {
                let passed = check_cc_vol(&symbol_set, 3);
                let msg = if passed {
                    "Volume lock satisfied."
                } else {
                    "Insufficient number of concrete function/struct declarations."
                };
                result.record_entry(
                    "CC-VOL",
                    passed,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-CRATE" => {
                let prev = req
                    .previous_symbols
                    .as_ref()
                    .map(|v| v.iter().cloned().collect::<HashSet<_>>())
                    .unwrap_or_else(HashSet::new);
                let (ok, new_symbols) = check_cc_crate(&prev, &symbol_set);
                let msg = if ok {
                    "Fresh symbols introduced compared to previous snapshot."
                } else {
                    "No new symbols introduced compared to previous snapshot."
                };
                result.record_entry(
                    "CC-CRATE",
                    ok,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if ok { Severity::Info } else { Severity::Warning },
                );
                if result.new_symbols.is_none() {
                    result.new_symbols = Some(new_symbols);
                }
            }
            "CC-NAV" => {
                let passed = check_cc_nav(&scan.navigation_candidates);
                let msg = if passed {
                    "Custom navigation function present and no external walker detected."
                } else {
                    "Custom navigation function not found or external walker detected."
                };
                result.record_entry(
                    "CC-NAV",
                    passed,
                    msg,
                    &path_for_entries,
                    0,
                    0,
                    if passed { Severity::Info } else { Severity::Error },
                );
            }
            "CC-BLACKLIST" => {
                // Already handled above in run_blacklist_scan.
            }
            // Unknown tags must be ignored per VALIDATOR-CONTRACT-1.
            _ => {}
        }
    }

    result
}

/* ---------- Helpers: JSON, tags, header, profiles ---------- */

/// Parse a simple JSON array of strings into Vec<String>.
///
/// Input form: ["TAG1","TAG2"] or with whitespace.
/// This is a minimal parser to avoid external dependencies.
fn parse_tags_json(tags_json: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;

    for ch in tags_json.chars() {
        if escape {
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
            _ if in_string => current.push(ch),
            _ => {}
        }
    }

    tags
}

/// Escape a string for JSON value position (no surrounding quotes).
fn escape_json_string(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Skip control chars in this subset.
            }
            other => out.push(other),
        }
    }
    out
}

/// Extract FILE header path from the first few lines of code.
/// Accepts headers like:
///   // FILE: ./src/core/engine/validator.rs
///   <!-- FILE: ./src/core/engine/validator.rs -->
///   FILE: ./src/core/engine/validator.rs
fn extract_file_header_path(code: &str) -> Option<String> {
    const MAX_HEADER_LINES: usize = 10;
    for (i, line) in code.lines().enumerate() {
        if i >= MAX_HEADER_LINES {
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

/// Build a ScanProfile from tag set.
fn build_scan_profile(tags: &[String]) -> ScanProfile {
    let mut want_decls = false;
    let mut want_imports = false;
    let mut want_nav = false;

    for tag in tags {
        match tag.as_str() {
            "CC-VOL" | "CC-CRATE" => want_decls = true,
            "CC-SOV" => want_imports = true,
            "CC-NAV" => want_nav = true,
            _ => {}
        }
    }

    let mut ids = Vec::new();
    if want_decls {
        ids.push(ScanProfileId::Declarations);
    }
    if want_imports {
        ids.push(ScanProfileId::Imports);
    }
    if want_nav {
        ids.push(ScanProfileId::Navigation);
    }

    ScanProfile { ids }
}

/* ---------- Individual CC- checks ---------- */

fn check_cc_lang(path: &str) -> bool {
    // Allowed extensions: .rs, .js, .cpp, .h, .aln, .md
    if let Some(idx) = path.rfind('.') {
        let ext = &path[idx + 1..];
        match ext {
            "rs" | "js" | "cpp" | "h" | "aln" | "md" => true,
            _ => false,
        }
    } else {
        false
    }
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

fn check_cc_path(path: &str) -> bool {
    if path.trim().is_empty() {
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

fn check_cc_deep(path: &str) -> bool {
    let norm = normalize_path(path);
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty()).collect();
    parts.len() >= 3
}

fn check_cc_zero(code: &str) -> bool {
    let banned = [
        "install ",
        "setup ",
        "setup(",
        "std::env::temp_dir",
        "std::env::var",
        "TEMP_DIR",
        "cargo install",
        "npm ",
        "yarn ",
        "pip ",
        "virtualenv",
    ];
    for pat in &banned {
        if code.contains(pat) {
            return false;
        }
    }
    true
}

fn check_cc_sov(import_lines: &[String]) -> bool {
    let banned = [
        "reqwest",
        "serde_json",
        "serde::",
        "tokio",
        "hyper",
        "openai",
        "axios",
        "tree-sitter",
        "syn",
    ];
    for line in import_lines {
        for pat in &banned {
            if line.contains(pat) {
                return false;
            }
        }
    }
    true
}

fn check_cc_vol(symbols: &HashSet<String>, min: usize) -> bool {
    symbols.len() >= min
}

fn check_cc_crate(
    previous: &HashSet<String>,
    current: &HashSet<String>,
) -> (bool, Vec<String>) {
    let mut new_syms = Vec::new();
    for sym in current {
        if !previous.contains(sym) {
            new_syms.push(sym.clone());
        }
    }
    let ok = !new_syms.is_empty();
    (ok, new_syms)
}

fn check_cc_nav(navigation_candidates: &[String]) -> bool {
    if navigation_candidates.is_empty() {
        return false;
    }
    let banned = ["walkdir", "globwalk"];
    for cand in navigation_candidates {
        for pat in &banned {
            if cand.contains(pat) {
                return false;
            }
        }
    }
    true
}

/* ---------- Path normalization and blacklist scan ---------- */

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for raw in path.split('/') {
        let segment = raw.trim();
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if !parts.is_empty() {
                parts.pop();
            }
            continue;
        }
        parts.push(segment);
    }
    parts.join("/")
}

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
        .unwrap_or_else(|| req.path.clone());

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
