// FILE: ./coreengine/src/tokenwalker.rs
//
// cc-token-walker: custom, zero-dependency token walker for Code-Command.
//
// This module has two distinct layers:
//
// 1. Bitfield-oriented ScanProfile (RQ-121–RQ-135)
//    - Encodes CC- tag activation, blacklist toggles, and LanguageHint
//      into a compact u32 mask.
//    - Used by Tier 1/Tier 2 validators to decide which checks are hot
//      for a given validation run.
//
// 2. Structural, line-oriented TokenWalker (RQ-136–RQ-150)
//    - Performs a single-pass scan over UTF-8 source text.
//    - Collects symbol declarations, import lines, and navigation
//      candidates with minimal, language-aware heuristics.
//
// Invariants (VALIDATOR-CONTRACT-1):
// - CC-LANG: sovereign stack only (Rust/JS/CPP/ALN/MD).
// - CC-SOV: no external parser crates (no syn, no tree-sitter, no regex).
// - CC-FULL: this file is complete; no omitted sections.
// - CC-PATH: no malformed paths in CC-FILE/FILE header logic upstream.
// - CC-NAV: navigation detection is implemented with custom heuristics only.

/// LanguageHint is used in two places:
/// - As a conceptual hint for TokenWalker heuristics.
/// - As a compact value encoded into ScanProfile.bits (bits 16..19).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LanguageHint {
    Rust = 0,
    Js   = 1,
    Cpp  = 2,
    Aln  = 3,
    Md   = 4,
}

// CC- tag bit flags (Tier 2 / profile layer).
pub const BIT_CC_FILE:   u32 = 1 << 0;
pub const BIT_CC_LANG:   u32 = 1 << 1;
pub const BIT_CC_FULL:   u32 = 1 << 2;
pub const BIT_CC_PATH:   u32 = 1 << 3;
pub const BIT_CC_DEEP:   u32 = 1 << 4;
pub const BIT_CC_SOV:    u32 = 1 << 5;
pub const BIT_CC_NAV:    u32 = 1 << 6;
pub const BIT_CC_VOL:    u32 = 1 << 7;
pub const BIT_CC_CRATE:  u32 = 1 << 8;

// LanguageHint encoded in bits 16..19.
const LANG_SHIFT: u32 = 16;
const LANG_MASK:  u32 = 0xF << LANG_SHIFT;

// Blacklist flags (pattern classes) in bits 20..23.
pub const BIT_BL_SOV_CRATES: u32 = 1 << 20; // reqwest, serde_json, openai, etc.
pub const BIT_BL_BLACKLIST:  u32 = 1 << 21; // (*/) hard blacklist names.

/// ScanProfile is the compact, bitfield representation used by the
/// validator to know which CC- tags and blacklist classes are active,
/// and which LanguageHint should be used by TokenWalker.
#[derive(Copy, Clone, Debug)]
pub struct ScanProfile {
    pub bits: u32,
}

/// High-level IDs for structural scan modes. These are used by the
/// line-oriented TokenWalker and are derived from ScanProfile.bits.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScanProfileId {
    Declarations,
    Imports,
    Navigation,
}

/// Language used by Symbol and other structural outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Unknown,
    Rust,
    Js,
    Cpp,
    Aln,
    Md,
}

/// SymbolKind classifies the type of declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    Struct,
    Class,
    Mod,
}

/// Location of a token or symbol in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub line:   u32, // 1-based
    pub column: u32, // 1-based, byte offset within line
}

/// Symbol is a declaration discovered by the walker.
/// This struct is ready to be attached to VFS/VFS snapshot paths.
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name:      String,
    pub kind:      SymbolKind,
    pub language:  Language,
    pub file_path: String,   // normalized path from FILE header / VFS
    pub location:  Location,
}

impl ScanProfile {
    /// Construct a ScanProfile from a tag list and language hint.
    ///
    /// Input:
    /// - tags: e.g. ["CC-FILE","CC-PATH","CC-SOV","CC-VOL"]
    /// - lang: language hint derived from CC-LANG or file extension.
    ///
    /// This encodes:
    /// - one bit per CC- tag
    /// - blacklist bits for CC-SOV / blacklist class
    /// - LanguageHint in bits 16..19
    pub fn for_tags_and_lang(tags: &[String], lang: LanguageHint) -> Self {
        let mut bits = 0u32;

        for t in tags {
            match t.as_str() {
                "CC-FILE"  => bits |= BIT_CC_FILE,
                "CC-LANG"  => bits |= BIT_CC_LANG,
                "CC-FULL"  => bits |= BIT_CC_FULL,
                "CC-PATH"  => bits |= BIT_CC_PATH,
                "CC-DEEP"  => bits |= BIT_CC_DEEP,
                "CC-SOV"   => {
                    // Sovereign-only: also enable sovereign crate blacklist.
                    bits |= BIT_CC_SOV | BIT_BL_SOV_CRATES;
                }
                "CC-NAV"   => bits |= BIT_CC_NAV,
                "CC-VOL"   => bits |= BIT_CC_VOL,
                "CC-CRATE" => bits |= BIT_CC_CRATE,
                _ => {}
            }
        }

        // Encode LanguageHint.
        bits |= (lang as u32) << LANG_SHIFT;

        // NOTE: if additional hard blacklist patterns are configured
        // in policy (e.g., (*/) names), the caller may OR in
        // BIT_BL_BLACKLIST before constructing this ScanProfile.

        ScanProfile { bits }
    }

    /// Decode the LanguageHint from the bitfield.
    pub fn language(&self) -> LanguageHint {
        let v = (self.bits & LANG_MASK) >> LANG_SHIFT;
        match v {
            0 => LanguageHint::Rust,
            1 => LanguageHint::Js,
            2 => LanguageHint::Cpp,
            3 => LanguageHint::Aln,
            4 => LanguageHint::Md,
            _ => LanguageHint::Rust,
        }
    }

    /// Check if a specific CC-/blacklist bit is set.
    #[inline]
    pub fn has(&self, mask: u32) -> bool {
        (self.bits & mask) != 0
    }

    /// Derive high-level structural profile IDs for the walker based
    /// on active CC- tags. This keeps the inner scan loop lean by only
    /// enabling patterns that are actually needed.
    pub fn structural_ids(&self) -> Vec<ScanProfileId> {
        let mut ids = Vec::new();

        // Declarations are needed for CC-VOL and CC-CRATE.
        if self.has(BIT_CC_VOL) || self.has(BIT_CC_CRATE) {
            ids.push(ScanProfileId::Declarations);
        }

        // Imports are needed for CC-SOV.
        if self.has(BIT_CC_SOV) {
            ids.push(ScanProfileId::Imports);
        }

        // Navigation is needed for CC-NAV.
        if self.has(BIT_CC_NAV) {
            ids.push(ScanProfileId::Navigation);
        }

        ids
    }
}

/// ScanResult holds aggregated structural information from a single scan.
#[derive(Clone, Debug)]
pub struct ScanResult {
    pub symbols: Vec<String>,
    pub import_lines: Vec<String>,
    pub navigation_candidates: Vec<String>,
}

/// TokenWalker performs a single-pass scan over UTF-8 source text.
/// It is intentionally simple: it only recognizes a few patterns that match
/// the CC- invariants (fn/struct/etc, imports, navigation).
pub struct TokenWalker<'a> {
    source:   &'a str,
    language: LanguageHint,
}

impl<'a> TokenWalker<'a> {
    pub fn new(source: &'a str, language: LanguageHint) -> Self {
        TokenWalker { source, language }
    }

    /// Perform a single-pass scan according to the structural profile IDs
    /// derived from a ScanProfile.
    pub fn scan(&self, profile: &ScanProfile) -> ScanResult {
        let structural = profile.structural_ids();

        let want_decls = structural.contains(&ScanProfileId::Declarations);
        let want_imports = structural.contains(&ScanProfileId::Imports);
        let want_nav = structural.contains(&ScanProfileId::Navigation);

        let mut symbols = Vec::new();
        let mut import_lines = Vec::new();
        let mut navigation_candidates = Vec::new();

        for line in self.source.lines() {
            let trimmed = line.trim_start();

            if want_decls && is_declaration_line(trimmed, self.language) {
                if let Some(sym) = extract_symbol_name(trimmed, self.language) {
                    symbols.push(sym);
                }
            }

            if want_imports && is_import_line(trimmed, self.language) {
                import_lines.push(trimmed.to_string());
            }

            if want_nav && is_navigation_candidate(trimmed, self.language) {
                navigation_candidates.push(trimmed.to_string());
            }
        }

        ScanResult {
            symbols,
            import_lines,
            navigation_candidates,
        }
    }
}

// ---- Line classifiers ----

fn is_declaration_line(line: &str, lang: LanguageHint) -> bool {
    match lang {
        LanguageHint::Rust => {
            line.starts_with("fn ")
                || line.starts_with("pub fn ")
                || line.starts_with("struct ")
                || line.starts_with("pub struct ")
                || line.starts_with("mod ")
                || line.starts_with("pub mod ")
                || line.starts_with("impl ")
        }
        LanguageHint::Js => {
            line.starts_with("function ")
                || line.starts_with("export function ")
                || line.starts_with("class ")
                || line.starts_with("export class ")
        }
        LanguageHint::Cpp => {
            // Very conservative: something like `Type::name(args)` or `name(args)`.
            line.contains("::") && line.contains('(') && line.contains(')')
        }
        LanguageHint::Aln | LanguageHint::Md => false,
    }
}

fn is_import_line(line: &str, lang: LanguageHint) -> bool {
    match lang {
        LanguageHint::Rust => {
            line.starts_with("use ")
                || line.starts_with("extern crate ")
        }
        LanguageHint::Js => {
            line.starts_with("import ")
                || line.starts_with("require(")
        }
        LanguageHint::Cpp => {
            line.starts_with("#include ")
        }
        LanguageHint::Aln | LanguageHint::Md => false,
    }
}

fn is_navigation_candidate(line: &str, lang: LanguageHint) -> bool {
    match lang {
        LanguageHint::Rust => {
            // Look for functions likely to traverse directories or paths.
            if !(line.starts_with("fn ") || line.starts_with("pub fn ")) {
                return false;
            }

            // Heuristic keywords for navigation functions.
            let has_nav_token =
                line.contains("walkdir")
                    || line.contains("walk_dir")
                    || line.contains("walk")
                    || line.contains("readdir")
                    || line.contains("read_dir");

            if !has_nav_token {
                return false;
            }

            // Heuristics: parameters referencing path or directory.
            line.contains("Path")
                || line.contains("path:")
                || line.contains("dir:")
        }
        LanguageHint::Js => {
            // Node.js / browser-style directory traversal.
            line.contains("readdir")
                || line.contains("walkDir")
        }
        LanguageHint::Cpp => {
            line.contains("readdir")
                || line.contains("filesystem")
        }
        LanguageHint::Aln | LanguageHint::Md => false,
    }
}

// ---- Symbol extraction helpers ----

fn extract_symbol_name(line: &str, lang: LanguageHint) -> Option<String> {
    match lang {
        LanguageHint::Rust => extract_rust_symbol_name(line),
        LanguageHint::Js   => extract_js_symbol_name(line),
        LanguageHint::Cpp  => extract_cpp_symbol_name(line),
        LanguageHint::Aln | LanguageHint::Md => None,
    }
}

fn extract_rust_symbol_name(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == '(' || c == '{')
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return None;
    }

    if tokens[0] == "pub" && tokens.len() >= 3 {
        match tokens[1] {
            "fn" | "struct" | "mod" => return Some(tokens[2].to_string()),
            _ => {}
        }
    }

    if tokens[0] == "fn" || tokens[0] == "struct" || tokens[0] == "mod" {
        if tokens.len() >= 2 {
            return Some(tokens[1].to_string());
        }
    }

    if tokens[0] == "impl" && tokens.len() >= 2 {
        return Some(tokens[1].to_string());
    }

    None
}

fn extract_js_symbol_name(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == '(' || c == '{')
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return None;
    }

    if tokens[0] == "export" && tokens.len() >= 3 {
        match tokens[1] {
            "function" | "class" => return Some(tokens[2].to_string()),
            _ => {}
        }
    }

    if tokens[0] == "function" && tokens.len() >= 2 {
        return Some(tokens[1].to_string());
    }

    if tokens[0] == "class" && tokens.len() >= 2 {
        return Some(tokens[1].to_string());
    }

    None
}

fn extract_cpp_symbol_name(line: &str) -> Option<String> {
    // Extremely conservative: take token before '(' as symbol candidate.
    if let Some(idx) = line.find('(') {
        let before = &line[..idx];
        let tokens: Vec<&str> = before
            .split(|c: char| c.is_whitespace() || c == ':' || c == '*')
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(last) = tokens.last() {
            return Some(last.to_string());
        }
    }
    None
}
