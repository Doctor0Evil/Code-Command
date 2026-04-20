// FILE .core/engine/src/tokenwalker.rs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    Struct,
    Class,
    Mod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Unknown,
    Rust,
    Js,
    Cpp,
    Aln,
    Md,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub line: u32,   // 1-based
    pub column: u32, // 1-based, byte offset within line
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub language: Language,
    pub file_path: String, // normalized path from FILE header / VFS
    pub location: Location,
}
