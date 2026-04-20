/// Blacklist rule definitions and rule set management
///
/// Defines the data structures for blacklist rules, their organization
/// into fast-lookup buckets, and the profile that aggregates all rules.

use crate::blacklist_pattern::{BlacklistPattern, MatchInfo, PatternError};
use std::collections::HashMap;
use std::fmt;

/// Language scope for a blacklist rule
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleLanguage {
    /// Rule applies to any language
    Any,
    /// Specific language (e.g., "rust", "javascript", "python")
    Specific(String),
}

impl RuleLanguage {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "any" | "*" => RuleLanguage::Any,
            other => RuleLanguage::Specific(other.to_string()),
        }
    }
    
    pub fn matches(&self, target: &RuleLanguage) -> bool {
        match self {
            RuleLanguage::Any => true,
            RuleLanguage::Specific(lang) => target == self,
        }
    }
}

/// Context where a rule applies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleContext {
    /// Any context (comments, strings, code, etc.)
    Any,
    /// Code only (excluding comments and strings)
    Code,
    /// Import/require/use statements
    Import,
    /// Declaration context
    Declaration,
    /// String literals only
    StringLiteral,
    /// Comments only
    Comment,
}

impl RuleContext {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "any" | "*" => RuleContext::Any,
            "code" => RuleContext::Code,
            "import" => RuleContext::Import,
            "declaration" => RuleContext::Declaration,
            "string" | "stringliteral" => RuleContext::StringLiteral,
            "comment" => RuleContext::Comment,
            _ => RuleContext::Any,
        }
    }
}

/// Severity level for a blacklist violation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// Block the operation entirely
    Block,
    /// Issue a warning but allow
    Warn,
    /// Just report for auditing
    Report,
}

impl Severity {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "block" | "error" => Severity::Block,
            "warn" | "warning" => Severity::Warn,
            "report" | "info" => Severity::Report,
            _ => Severity::Block, // Default to strict
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            Severity::Block => "block".to_string(),
            Severity::Warn => "warn".to_string(),
            Severity::Report => "report".to_string(),
        }
    }
}

/// A single blacklist rule
#[derive(Debug, Clone)]
pub struct BlacklistRule {
    /// Unique identifier for the rule
    pub id: String,
    /// The token or pattern to match
    pub token: String,
    /// Compiled pattern (if token uses regex syntax)
    pub pattern: Option<BlacklistPattern>,
    /// Language this rule applies to
    pub language: RuleLanguage,
    /// Context this rule applies to
    pub context: RuleContext,
    /// Severity level
    pub severity: Severity,
    /// Reason/explanation for the rule
    pub reason: String,
    /// Whether this is a pattern (true) or literal match (false)
    pub is_pattern: bool,
}

impl BlacklistRule {
    pub fn new(
        id: String,
        token: String,
        language: RuleLanguage,
        context: RuleContext,
        severity: Severity,
        reason: String,
    ) -> Result<Self, PatternError> {
        let is_pattern = Self::looks_like_pattern(&token);
        let pattern = if is_pattern {
            Some(BlacklistPattern::parse(&token)?)
        } else {
            None
        };
        
        Ok(BlacklistRule {
            id,
            token,
            pattern,
            language,
            context,
            severity,
            reason,
            is_pattern,
        })
    }
    
    /// Heuristic to detect if token should be treated as a pattern
    fn looks_like_pattern(token: &str) -> bool {
        token.chars().any(|c| matches!(c, '.' | '*' | '+' | '?' | '[' | '^' | '$' | '|' | '('))
    }
    
    /// Check if this rule matches the given input
    pub fn matches(&self, input: &[u8]) -> Option<MatchInfo> {
        if self.is_pattern {
            self.pattern.as_ref()?.matches(input)
        } else {
            // Literal substring search
            let token_bytes = self.token.as_bytes();
            input
                .windows(token_bytes.len())
                .position(|window| window == token_bytes)
                .map(|start| MatchInfo {
                    start,
                    end: start + token_bytes.len(),
                    captures: HashMap::new(),
                })
        }
    }
}

/// Bucket key for rule organization
type RuleBucketKey = (RuleLanguage, RuleContext);

/// Organized set of blacklist rules for fast lookup
#[derive(Debug, Clone)]
pub struct BlacklistRuleSet {
    /// Rules organized by (language, context) buckets
    buckets: HashMap<RuleBucketKey, Vec<BlacklistRule>>,
    /// All rules in a flat list (for iteration)
    all_rules: Vec<BlacklistRule>,
    /// Hard markers (literal tokens) for fast Tier-1 scanning
    hard_markers: HashMap<String, Vec<usize>>, // token -> rule indices
}

impl BlacklistRuleSet {
    pub fn new() -> Self {
        BlacklistRuleSet {
            buckets: HashMap::new(),
            all_rules: Vec::new(),
            hard_markers: HashMap::new(),
        }
    }
    
    /// Add a rule to the rule set
    pub fn add_rule(&mut self, rule: BlacklistRule) {
        let rule_idx = self.all_rules.len();
        
        // Add to flat list
        self.all_rules.push(rule.clone());
        
        // Add to appropriate bucket
        let key = (rule.language.clone(), rule.context.clone());
        self.buckets.entry(key).or_insert_with(Vec::new).push(rule.clone());
        
        // If it's a literal (not pattern), add to hard markers for fast scanning
        if !rule.is_pattern {
            self.hard_markers
                .entry(rule.token.clone())
                .or_insert_with(Vec::new)
                .push(rule_idx);
        }
        
        // For language-agnostic rules, also add to RuleLanguage::Any buckets
        if rule.language == RuleLanguage::Any {
            for (lang, _) in [&RuleContext::Any, &RuleContext::Code, &RuleContext::Import] {
                let any_lang_key = (RuleLanguage::Any, lang.clone());
                if !self.buckets.contains_key(&any_lang_key) {
                    self.buckets.insert(any_lang_key.clone(), Vec::new());
                }
            }
        }
    }
    
    /// Get rules for a specific language and context
    pub fn get_rules(&self, language: &RuleLanguage, context: &RuleContext) -> Vec<&BlacklistRule> {
        let mut rules = Vec::new();
        
        // Try exact match first
        let key = (language.clone(), context.clone());
        if let Some(bucket) = self.buckets.get(&key) {
            rules.extend(bucket.iter());
        }
        
        // Fallback to language-agnostic rules
        if language != &RuleLanguage::Any {
            let any_lang_key = (RuleLanguage::Any, context.clone());
            if let Some(bucket) = self.buckets.get(&any_lang_key) {
                rules.extend(bucket.iter());
            }
        }
        
        // Fallback to context-agnostic rules
        if context != &RuleContext::Any {
            let any_ctx_key = (language.clone(), RuleContext::Any);
            if let Some(bucket) = self.buckets.get(&any_ctx_key) {
                rules.extend(bucket.iter());
            }
        }
        
        // Fallback to completely agnostic rules
        let any_any_key = (RuleLanguage::Any, RuleContext::Any);
        if let Some(bucket) = self.buckets.get(&any_any_key) {
            rules.extend(bucket.iter());
        }
        
        rules
    }
    
    /// Get all hard markers for fast Tier-1 scanning
    pub fn get_hard_markers(&self) -> &HashMap<String, Vec<usize>> {
        &self.hard_markers
    }
    
    /// Get all rules
    pub fn all_rules(&self) -> &[BlacklistRule] {
        &self.all_rules
    }
    
    /// Build from a list of rules
    pub fn from_rules(rules: Vec<BlacklistRule>) -> Self {
        let mut rule_set = BlacklistRuleSet::new();
        for rule in rules {
            rule_set.add_rule(rule);
        }
        rule_set
    }
}

impl Default for BlacklistRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate configuration for blacklist scanning
#[derive(Debug, Clone)]
pub struct BlacklistProfile {
    /// All rules in priority order
    pub rules: Vec<BlacklistRule>,
    /// Organized rule set for fast lookup
    pub rule_set: BlacklistRuleSet,
    /// Precomputed hard marker set for Tier-1 scanning
    pub hard_marker_set: std::collections::HashSet<String>,
}

/// Error types for loading blacklist profiles
#[derive(Debug)]
pub enum BlacklistLoadError {
    VfsReadFailed(String),
    ParseError(String),
}

impl BlacklistProfile {
    pub fn new(rules: Vec<BlacklistRule>) -> Self {
        let rule_set = BlacklistRuleSet::from_rules(rules.clone());
        let hard_marker_set: std::collections::HashSet<String> = rules
            .iter()
            .filter(|r| !r.is_pattern)
            .map(|r| r.token.clone())
            .collect();
        
        BlacklistProfile {
            rules,
            rule_set,
            hard_marker_set,
        }
    }
    
    /// Check if a string contains any hard marker (fast path)
    pub fn has_hard_marker(&self, content: &str) -> bool {
        for marker in &self.hard_marker_set {
            if content.contains(marker) {
                return true;
            }
        }
        false
    }
}

impl Default for BlacklistProfile {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Canonical entry point to load and merge blacklist policies from VFS
///
/// AI-Chat Usage:
/// ```rust
/// let profile = load_blacklist_profile(&vfs)?;
/// let validator = Validator::with_blacklist(..., profile);
/// ```
pub fn load_blacklist_profile(vfs: &crate::vfs::Vfs) -> Result<BlacklistProfile, BlacklistLoadError> {
    // 1. Load global spec
    let global_content = vfs.read("specs/blacklist.aln")
        .map_err(|e| BlacklistLoadError::VfsReadFailed(format!("specs/blacklist.aln: {}", e)))?;
    
    let mut rules = parse_blacklist_aln(&global_content)?;

    // 2. Load repo-local overrides (optional)
    if let Ok(local_content) = vfs.read(".ccblacklist.aln") {
        let local_rules = parse_blacklist_aln(&local_content)?;
        merge_rules(&mut rules, local_rules);
    }

    Ok(BlacklistProfile::new(rules))
}

/// Parses an ALN document into a list of BlacklistRules
/// Expects a `blacklist:` key followed by a list of maps with id, token/pattern, language, severity, reason
pub fn parse_blacklist_aln(doc: &str) -> Result<Vec<BlacklistRule>, BlacklistLoadError> {
    let mut rules = Vec::new();
    let mut in_block = false;
    let mut current_rule: std::collections::HashMap<&str, String> = std::collections::HashMap::new();

    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { 
            continue; 
        }

        if trimmed == "blacklist:" {
            in_block = true;
            continue;
        }

        if in_block {
            if trimmed.starts_with("- id:") {
                // Save previous rule if exists
                if !current_rule.is_empty() {
                    if let Some(rule) = build_rule_from_map(current_rule) {
                        rules.push(rule);
                    }
                }
                current_rule = std::collections::HashMap::new();
                current_rule.insert("id", trimmed[5..].trim().trim_matches('"').to_string());
            } else if trimmed.starts_with("token:") || trimmed.starts_with("pattern:") {
                let (k, v) = trimmed.split_once(':').unwrap();
                current_rule.insert(k.trim(), v.trim().trim_matches('"').to_string());
            } else if trimmed.starts_with("language:") || trimmed.starts_with("severity:") || trimmed.starts_with("reason:") || trimmed.starts_with("context:") {
                let (k, v) = trimmed.split_once(':').unwrap();
                current_rule.insert(k.trim(), v.trim().trim_matches('"').to_string());
            } else if !trimmed.starts_with(' ') && !trimmed.starts_with('-') && !trimmed.starts_with('\t') {
                // End of block (new top-level key)
                in_block = false;
            }
        }
    }
    
    // Push last rule
    if !current_rule.is_empty() {
        if let Some(rule) = build_rule_from_map(current_rule) {
            rules.push(rule);
        }
    }

    Ok(rules)
}

fn build_rule_from_map(map: std::collections::HashMap<&str, String>) -> Option<BlacklistRule> {
    let id = map.get("id")?.clone();
    let token = map.get("token").cloned().or_else(|| map.get("pattern").cloned())?;
    let language = RuleLanguage::from_str(map.get("language").cloned().unwrap_or_else(|| "any".to_string()).as_str());
    let context = RuleContext::from_str(map.get("context").cloned().unwrap_or_else(|| "any".to_string()).as_str());
    let severity = Severity::from_str(map.get("severity").cloned().unwrap_or_else(|| "warn".to_string()).as_str());
    let reason = map.get("reason").cloned().unwrap_or_default();

    BlacklistRule::new(id, token, language, context, severity, reason).ok()
}

fn merge_rules(global: &mut Vec<BlacklistRule>, local: Vec<BlacklistRule>) {
    for rule in local {
        if let Some(existing) = global.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule; // Override
        } else {
            global.push(rule); // Append
        }
    }
}

/// High-level scan entry point used by Validator and TaskQueue
/// Returns a list of contamination reports found in the content
pub fn scan_content(
    profile: &BlacklistProfile,
    code: &str,
    path: &str,
    language: crate::language::LanguageHint,
    context: RuleContext,
) -> Vec<crate::validator::ContaminationReport> {
    use crate::validator::ContaminationReport;
    
    let mut reports = Vec::new();
    let rule_lang = match language {
        crate::language::LanguageHint::Rust => RuleLanguage::Specific("rust".to_string()),
        crate::language::LanguageHint::JavaScript => RuleLanguage::Specific("javascript".to_string()),
        crate::language::LanguageHint::Python => RuleLanguage::Specific("python".to_string()),
        crate::language::LanguageHint::Unknown => RuleLanguage::Any,
    };

    // Tier 1: Fast literal check against hard markers
    if profile.has_hard_marker(code) {
        // Proceed to full scan
    } else {
        // Quick exit if no hard markers and we only care about blocks
        // (In production, still scan for warn/report rules)
    }

    // Tier 2: Full scan via RuleSet
    let candidates = profile.rule_set.get_rules(&rule_lang, &context);
    for rule in candidates {
        if let Some(_match_info) = rule.matches(code.as_bytes()) {
            // Calculate line/col (simplified)
            let idx = _match_info.start;
            let line = code[..idx].lines().count() + 1;
            let col = code[..idx].lines().last().map(|l| l.len()).unwrap_or(0) + 1;
            
            reports.push(ContaminationReport {
                pattern: rule.token.clone(),
                exact_match: true,
                surrounding_context: "".to_string(), // Could extract snippet
                severity: rule.severity.to_string(),
                path: path.to_string(),
                line,
                column: col,
            });
            
            if rule.severity == Severity::Block {
                // Policy might dictate short-circuit on first block
                break; 
            }
        }
    }

    reports
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rule_creation() {
        let rule = BlacklistRule::new(
            "BL-001".to_string(),
            "eval".to_string(),
            RuleLanguage::Any,
            RuleContext::Code,
            Severity::Block,
            "Forbidden function".to_string(),
        ).unwrap();
        
        assert_eq!(rule.id, "BL-001");
        assert!(!rule.is_pattern);
        assert!(rule.matches(b"function eval()").is_some());
    }
    
    #[test]
    fn test_pattern_rule() {
        let rule = BlacklistRule::new(
            "BL-002".to_string(),
            "eval\\s*\\(".to_string(),
            RuleLanguage::Any,
            RuleContext::Code,
            Severity::Block,
            "Forbidden eval call".to_string(),
        ).unwrap();
        
        assert!(rule.is_pattern);
        assert!(rule.matches(b"eval(").is_some());
        assert!(rule.matches(b"eval (").is_some());
    }
    
    #[test]
    fn test_rule_set_bucketing() {
        let mut rule_set = BlacklistRuleSet::new();
        
        let rust_rule = BlacklistRule::new(
            "BL-RUST".to_string(),
            "unsafe".to_string(),
            RuleLanguage::Specific("rust".to_string()),
            RuleContext::Code,
            Severity::Warn,
            "Unsafe code".to_string(),
        ).unwrap();
        
        let any_rule = BlacklistRule::new(
            "BL-ANY".to_string(),
            "TODO".to_string(),
            RuleLanguage::Any,
            RuleContext::Any,
            Severity::Report,
            "TODO comment".to_string(),
        ).unwrap();
        
        rule_set.add_rule(rust_rule);
        rule_set.add_rule(any_rule);
        
        // Should find both rules for Rust code
        let rust_rules = rule_set.get_rules(
            &RuleLanguage::Specific("rust".to_string()),
            &RuleContext::Code,
        );
        assert_eq!(rust_rules.len(), 2);
        
        // Should find only any_rule for Python code
        let python_rules = rule_set.get_rules(
            &RuleLanguage::Specific("python".to_string()),
            &RuleContext::Code,
        );
        assert_eq!(python_rules.len(), 1);
    }
}
