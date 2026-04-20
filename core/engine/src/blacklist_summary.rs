/// Blacklist summary report generation
///
/// Aggregates BlacklistMatch instances into a compact summary
/// for CI dashboards and TaskReport attachment.

use crate::blacklist_cache::BlacklistMatch;
use std::collections::HashMap;

/// Severity counts in the summary
#[derive(Debug, Clone, Default)]
pub struct SeverityCounts {
    pub block: u32,
    pub warn: u32,
    pub report: u32,
}

/// Summary of blacklist violations aggregated across files
#[derive(Debug, Clone)]
pub struct BlacklistSummary {
    /// Total number of violations found
    pub total_violations: u32,
    /// Counts by severity level
    pub severity_counts: SeverityCounts,
    /// Counts by rule ID
    pub by_rule: HashMap<String, u32>,
    /// Counts by file path
    pub by_file: HashMap<String, u32>,
    /// Detailed matches grouped by file
    pub details: HashMap<String, Vec<BlacklistMatch>>,
}

impl BlacklistSummary {
    pub fn new() -> Self {
        BlacklistSummary {
            total_violations: 0,
            severity_counts: SeverityCounts::default(),
            by_rule: HashMap::new(),
            by_file: HashMap::new(),
            details: HashMap::new(),
        }
    }

    /// Aggregate matches from multiple files
    /// 
    /// `file_matches` is a vector of (path, matches) pairs
    pub fn aggregate(file_matches: Vec<(String, Vec<BlacklistMatch>)>) -> Self {
        let mut summary = BlacklistSummary::new();

        for (path, matches) in file_matches {
            for m in &matches {
                summary.total_violations += 1;

                // Update severity counts
                match m.severity.as_str() {
                    "block" => summary.severity_counts.block += 1,
                    "warn" => summary.severity_counts.warn += 1,
                    "report" => summary.severity_counts.report += 1,
                    _ => {}
                }

                // Update by_rule counts
                *summary.by_rule.entry(m.rule_id.clone()).or_insert(0) += 1;

                // Update by_file counts
                *summary.by_file.entry(path.clone()).or_insert(0) += 1;
            }

            // Store details if there are any matches
            if !matches.is_empty() {
                summary.details.insert(path, matches);
            }
        }

        summary
    }

    /// Serialize to JSON with kind field for TaskReport attachment
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        
        // kind field
        out.push_str("\"kind\":\"BLACKLIST-SUMMARY-1\"");
        
        // total_violations
        out.push_str(",\"total_violations\":");
        out.push_str(&self.total_violations.to_string());
        
        // severity_counts
        out.push_str(",\"severity_counts\":{");
        out.push_str("\"block\":");
        out.push_str(&self.severity_counts.block.to_string());
        out.push_str(",\"warn\":");
        out.push_str(&self.severity_counts.warn.to_string());
        out.push_str(",\"report\":");
        out.push_str(&self.severity_counts.report.to_string());
        out.push('}');
        
        // by_rule
        out.push_str(",\"by_rule\":{");
        let mut first = true;
        for (rule_id, count) in &self.by_rule {
            if !first {
                out.push(',');
            }
            first = false;
            out.push('"');
            out.push_str(&escape_json(rule_id));
            out.push_str("\":");
            out.push_str(&count.to_string());
        }
        out.push('}');
        
        // by_file
        out.push_str(",\"by_file\":{");
        let mut first = true;
        for (path, count) in &self.by_file {
            if !first {
                out.push(',');
            }
            first = false;
            out.push('"');
            out.push_str(&escape_json(path));
            out.push_str("\":");
            out.push_str(&count.to_string());
        }
        out.push('}');
        
        // Note: details are omitted from compact summary JSON
        // They can be included separately if needed
        
        out.push('}');
        out
    }
    
    /// Check if there are any block-level violations
    pub fn has_block_violations(&self) -> bool {
        self.severity_counts.block > 0
    }
    
    /// Get all block-level matches for immediate incident reporting
    pub fn get_block_matches(&self) -> Vec<&BlacklistMatch> {
        let mut block_matches = Vec::new();
        for (_, matches) in &self.details {
            for m in matches {
                if m.severity == "block" {
                    block_matches.push(m);
                }
            }
        }
        block_matches
    }
}

impl Default for BlacklistSummary {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_aggregate_empty() {
        let summary = BlacklistSummary::aggregate(Vec::new());
        assert_eq!(summary.total_violations, 0);
        assert_eq!(summary.severity_counts.block, 0);
    }

    #[test]
    fn test_aggregate_single_match() {
        let matches = vec![
            BlacklistMatch {
                rule_id: "RULE-001".to_string(),
                pattern: "eval(".to_string(),
                start: 10,
                end: 15,
                severity: "block".to_string(),
                reason: "Dangerous".to_string(),
            }
        ];
        
        let file_matches = vec![("src/test.js".to_string(), matches)];
        let summary = BlacklistSummary::aggregate(file_matches);
        
        assert_eq!(summary.total_violations, 1);
        assert_eq!(summary.severity_counts.block, 1);
        assert_eq!(summary.by_rule.get("RULE-001"), Some(&1));
        assert_eq!(summary.by_file.get("src/test.js"), Some(&1));
    }

    #[test]
    fn test_aggregate_multiple_files() {
        let matches1 = vec![
            BlacklistMatch {
                rule_id: "RULE-001".to_string(),
                pattern: "eval(".to_string(),
                start: 10,
                end: 15,
                severity: "block".to_string(),
                reason: "Dangerous".to_string(),
            }
        ];
        
        let matches2 = vec![
            BlacklistMatch {
                rule_id: "RULE-002".to_string(),
                pattern: "dangerous_fn".to_string(),
                start: 20,
                end: 32,
                severity: "warn".to_string(),
                reason: "Risky".to_string(),
            }
        ];
        
        let file_matches = vec![
            ("src/a.js".to_string(), matches1),
            ("src/b.js".to_string(), matches2),
        ];
        let summary = BlacklistSummary::aggregate(file_matches);
        
        assert_eq!(summary.total_violations, 2);
        assert_eq!(summary.severity_counts.block, 1);
        assert_eq!(summary.severity_counts.warn, 1);
        assert_eq!(summary.by_rule.len(), 2);
        assert_eq!(summary.by_file.len(), 2);
    }

    #[test]
    fn test_has_block_violations() {
        let matches = vec![
            BlacklistMatch {
                rule_id: "RULE-001".to_string(),
                pattern: "eval(".to_string(),
                start: 10,
                end: 15,
                severity: "block".to_string(),
                reason: "Dangerous".to_string(),
            }
        ];
        
        let file_matches = vec![("src/test.js".to_string(), matches)];
        let summary = BlacklistSummary::aggregate(file_matches);
        
        assert!(summary.has_block_violations());
    }
}
