// FILE: ./core/engine/src/blacklist.rs

/// How the engine should react when a blacklist rule matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlacklistAction {
    /// Reject the operation and surface a hard validation / contamination failure.
    Block,
    /// Allow the operation but record a warning in the validation report.
    Warn,
}

/// Which language a rule applies to.
/// This mirrors the LanguageHint variants but keeps the blacklist decoupled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlacklistLanguage {
    Any,
    Rust,
    Js,
    Cpp,
    Aln,
    Md,
}

impl BlacklistLanguage {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "rust" => BlacklistLanguage::Rust,
            "js" | "javascript" => BlacklistLanguage::Js,
            "cpp" | "c++" => BlacklistLanguage::Cpp,
            "aln" => BlacklistLanguage::Aln,
            "md" | "markdown" => BlacklistLanguage::Md,
            "any" | "" => BlacklistLanguage::Any,
            _ => BlacklistLanguage::Any,
        }
    }
}

/// Raw pattern and metadata as loaded from ALN.
#[derive(Clone, Debug)]
pub struct BlacklistRule {
    /// Stable identifier, e.g. "BL-0001".
    pub id: String,

    /// Raw pattern text as written in ALN.
    pub pattern: String,

    /// If true, `pattern` is interpreted as a small regex subset;
    /// if false, it is a literal substring.
    pub is_regex: bool,

    /// Languages this rule applies to; empty or containing `Any` => all languages.
    pub languages: Vec<BlacklistLanguage>,

    /// Block or Warn behavior.
    pub action: BlacklistAction,

    /// Short human‑readable reason to surface in reports.
    pub reason: String,
}

/// Allows a specific path prefix to bypass selected blacklist rules.
#[derive(Clone, Debug)]
pub struct BlacklistExemption {
    /// Normalized VFS path prefix, e.g. ".specs/third-party".
    pub path_prefix: String,

    /// List of rule IDs this exemption disables under `path_prefix`.
    pub rule_ids: Vec<String>,
}

/// Top‑level blacklist configuration loaded from policy/specs.
#[derive(Clone, Debug)]
pub struct BlacklistConfig {
    /// All active blacklist rules.
    pub rules: Vec<BlacklistRule>,

    /// Per‑path exemptions that relax specific rules.
    pub exemptions: Vec<BlacklistExemption>,
}

impl BlacklistConfig {
    /// Returns an iterator over rules that are applicable to the given language,
    /// after applying any exemptions for the provided path.
    pub fn effective_rules<'a>(
        &'a self,
        lang: BlacklistLanguage,
        path: &str,
    ) -> impl Iterator<Item = &'a BlacklistRule> {
        use std::collections::HashSet;

        // Collect IDs of rules that are exempted for this path.
        let mut exempt_ids: HashSet<&str> = HashSet::new();
        for ex in &self.exemptions {
            if path.starts_with(&ex.path_prefix) {
                for id in &ex.rule_ids {
                    exempt_ids.insert(id.as_str());
                }
            }
        }

        self.rules.iter().filter(move |rule| {
            if exempt_ids.contains(rule.id.as_str()) {
                return false;
            }

            if rule.languages.is_empty() {
                return true;
            }

            rule.languages.iter().any(|&l| {
                l == BlacklistLanguage::Any || l == lang
            })
        })
    }
}
