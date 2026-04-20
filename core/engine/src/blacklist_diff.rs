/// Blacklist rule diffing for policy version comparison
///
/// Compares two sets of blacklist rules to identify additions,
/// removals, and modifications between policy versions.

use crate::blacklist::BlacklistRule;
use std::collections::HashMap;

/// Represents changes to a single rule (field-level differences)
#[derive(Debug, Clone, PartialEq)]
pub struct RuleModification {
    pub id: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// Aggregate diff result comparing two rule sets
#[derive(Debug, Clone, PartialEq)]
pub struct BlacklistDiff {
    /// Rule IDs that were added in the new set
    pub added_ids: Vec<String>,
    /// Rule IDs that were removed from the old set
    pub removed_ids: Vec<String>,
    /// Rules that exist in both but have modifications
    pub modified: Vec<RuleModification>,
}

impl BlacklistDiff {
    pub fn new() -> Self {
        BlacklistDiff {
            added_ids: Vec::new(),
            removed_ids: Vec::new(),
            modified: Vec::new(),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        
        // added_ids
        out.push_str("\"added_ids\":[");
        for (i, id) in self.added_ids.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(id));
            out.push('"');
        }
        out.push(']');
        
        // removed_ids
        out.push_str(",\"removed_ids\":[");
        for (i, id) in self.removed_ids.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(id));
            out.push('"');
        }
        out.push(']');
        
        // modified
        out.push_str(",\"modified\":[");
        for (i, modif) in self.modified.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"id\":\"");
            out.push_str(&escape_json(&modif.id));
            out.push_str("\",\"field\":\"");
            out.push_str(&escape_json(&modif.field));
            out.push_str("\",\"old_value\":\"");
            out.push_str(&escape_json(&modif.old_value));
            out.push_str("\",\"new_value\":\"");
            out.push_str(&escape_json(&modif.new_value));
            out.push('"');
            out.push('}');
        }
        out.push(']');
        
        out.push('}');
        out
    }
}

impl Default for BlacklistDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two rule sets and produce a diff
pub fn diff_rules(old: &[BlacklistRule], new: &[BlacklistRule]) -> BlacklistDiff {
    let mut diff = BlacklistDiff::new();
    
    // Build maps keyed by rule ID
    let mut old_map: HashMap<&str, &BlacklistRule> = HashMap::new();
    let mut new_map: HashMap<&str, &BlacklistRule> = HashMap::new();
    
    for rule in old {
        old_map.insert(&rule.id, rule);
    }
    for rule in new {
        new_map.insert(&rule.id, rule);
    }
    
    // Find added and modified rules
    for (id, new_rule) in &new_map {
        match old_map.get(id) {
            None => {
                // Rule was added
                diff.added_ids.push((*id).to_string());
            }
            Some(old_rule) => {
                // Check for modifications
                compare_rules(old_rule, new_rule, &mut diff);
            }
        }
    }
    
    // Find removed rules
    for (id, _) in &old_map {
        if !new_map.contains_key(id) {
            diff.removed_ids.push((*id).to_string());
        }
    }
    
    diff
}

/// Compare two rules field-by-field to detect modifications
fn compare_rules(old: &BlacklistRule, new: &BlacklistRule, diff: &mut BlacklistDiff) {
    // Compare token
    if old.token != new.token {
        diff.modified.push(RuleModification {
            id: old.id.clone(),
            field: "token".to_string(),
            old_value: old.token.clone(),
            new_value: new.token.clone(),
        });
    }
    
    // Compare language
    let old_lang = rule_language_to_string(&old.language);
    let new_lang = rule_language_to_string(&new.language);
    if old_lang != new_lang {
        diff.modified.push(RuleModification {
            id: old.id.clone(),
            field: "language".to_string(),
            old_value: old_lang,
            new_value: new_lang,
        });
    }
    
    // Compare context
    let old_ctx = rule_context_to_string(&old.context);
    let new_ctx = rule_context_to_string(&new.context);
    if old_ctx != new_ctx {
        diff.modified.push(RuleModification {
            id: old.id.clone(),
            field: "context".to_string(),
            old_value: old_ctx,
            new_value: new_ctx,
        });
    }
    
    // Compare severity
    let old_sev = old.severity.to_string();
    let new_sev = new.severity.to_string();
    if old_sev != new_sev {
        diff.modified.push(RuleModification {
            id: old.id.clone(),
            field: "severity".to_string(),
            old_value: old_sev,
            new_value: new_sev,
        });
    }
    
    // Compare reason
    if old.reason != new.reason {
        diff.modified.push(RuleModification {
            id: old.id.clone(),
            field: "reason".to_string(),
            old_value: old.reason.clone(),
            new_value: new.reason.clone(),
        });
    }
}

fn rule_language_to_string(lang: &crate::blacklist::RuleLanguage) -> String {
    match lang {
        crate::blacklist::RuleLanguage::Any => "any".to_string(),
        crate::blacklist::RuleLanguage::Specific(s) => s.clone(),
    }
}

fn rule_context_to_string(ctx: &crate::blacklist::RuleContext) -> String {
    match ctx {
        crate::blacklist::RuleContext::Any => "any".to_string(),
        crate::blacklist::RuleContext::Code => "code".to_string(),
        crate::blacklist::RuleContext::Import => "import".to_string(),
        crate::blacklist::RuleContext::Declaration => "declaration".to_string(),
        crate::blacklist::RuleContext::StringLiteral => "string".to_string(),
        crate::blacklist::RuleContext::Comment => "comment".to_string(),
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blacklist::{BlacklistRule, RuleLanguage, RuleContext, Severity};

    #[test]
    fn test_diff_added_rule() {
        let old: Vec<BlacklistRule> = Vec::new();
        let new = vec![
            BlacklistRule::new(
                "RULE-001".to_string(),
                "eval(".to_string(),
                RuleLanguage::Specific("javascript".to_string()),
                RuleContext::Code,
                Severity::Block,
                "Dangerous eval usage".to_string(),
            ).unwrap()
        ];
        
        let diff = diff_rules(&old, &new);
        assert_eq!(diff.added_ids.len(), 1);
        assert_eq!(diff.added_ids[0], "RULE-001");
        assert!(diff.removed_ids.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn test_diff_removed_rule() {
        let old = vec![
            BlacklistRule::new(
                "RULE-001".to_string(),
                "eval(".to_string(),
                RuleLanguage::Specific("javascript".to_string()),
                RuleContext::Code,
                Severity::Block,
                "Dangerous eval usage".to_string(),
            ).unwrap()
        ];
        let new: Vec<BlacklistRule> = Vec::new();
        
        let diff = diff_rules(&old, &new);
        assert!(diff.added_ids.is_empty());
        assert_eq!(diff.removed_ids.len(), 1);
        assert_eq!(diff.removed_ids[0], "RULE-001");
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn test_diff_modified_rule() {
        let old = vec![
            BlacklistRule::new(
                "RULE-001".to_string(),
                "eval(".to_string(),
                RuleLanguage::Specific("javascript".to_string()),
                RuleContext::Code,
                Severity::Warn,
                "Dangerous eval usage".to_string(),
            ).unwrap()
        ];
        let new = vec![
            BlacklistRule::new(
                "RULE-001".to_string(),
                "eval(".to_string(),
                RuleLanguage::Specific("javascript".to_string()),
                RuleContext::Code,
                Severity::Block,
                "Dangerous eval usage".to_string(),
            ).unwrap()
        ];
        
        let diff = diff_rules(&old, &new);
        assert!(diff.added_ids.is_empty());
        assert!(diff.removed_ids.is_empty());
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].field, "severity");
        assert_eq!(diff.modified[0].old_value, "warn");
        assert_eq!(diff.modified[0].new_value, "block");
    }
}
