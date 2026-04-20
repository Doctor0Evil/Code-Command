// FILE .core/engine/src/blacklist.rs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlacklistAction {
    Block,
    Warn,
}

#[derive(Clone, Debug)]
pub struct BlacklistRule {
    pub pattern: String,          // raw pattern as written in ALN
    pub is_regex: bool,           // true => interpret pattern as a small regex subset
    pub languages: Vec<String>,   // e.g. ["Rust", "Js"]; empty => all languages
    pub action: BlacklistAction,  // Block or Warn
}
