// FILE .coreengine/src/wiring-graph.rs

//! Wiring graph construction and validation for CC-WIRING checks.
//! 
//! This module provides sovereign, dependency-free wiring analysis that:
//! - Builds a graph of cross-module calls in the engine
//! - Validates against specs/wiring-spec.aln
//! - Detects forbidden edges and missing connections
//! - Emits WiringGraph JSON for ResearchObjects

use std::collections::HashMap;

/// A node in the wiring graph (typically a core engine component)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WiringNode {
    pub id: String,
}

impl WiringNode {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

/// An edge representing a call or dependency between nodes
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WiringEdge {
    pub from: String,      // source node id
    pub to: String,        // target node id
    pub kind: String,      // e.g. "call", "dependency"
    pub via: String,       // function/method name or pattern
    pub location: String,  // file:line where the edge was detected
}

impl WiringEdge {
    pub fn new(from: &str, to: &str, via: &str, location: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            kind: "call".to_string(),
            via: via.to_string(),
            location: location.to_string(),
        }
    }
}

/// The complete wiring graph for the engine
#[derive(Clone, Debug, Default)]
pub struct WiringGraph {
    pub nodes: Vec<WiringNode>,
    pub edges: Vec<WiringEdge>,
}

impl WiringGraph {
    pub fn new() -> Self {
        // Initialize with known core engine nodes
        let nodes = vec![
            WiringNode::new("Lib"),
            WiringNode::new("Validator"),
            WiringNode::new("Vfs"),
            WiringNode::new("Navigator"),
            WiringNode::new("TaskQueue"),
            WiringNode::new("TokenWalker"),
        ];
        Self { nodes, edges: Vec::new() }
    }

    pub fn add_edge(&mut self, from: &str, to: &str, via: &str, location: &str) {
        self.edges.push(WiringEdge::new(from, to, via, location));
    }

    /// Check if a node exists
    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.iter().any(|n| n.id == id)
    }

    /// Check if an edge exists (with pattern matching on via)
    pub fn has_edge(&self, from: &str, to: &str, via_pattern: &str) -> bool {
        self.edges.iter().any(|e| {
            e.from == from && e.to == to && e.via.contains(via_pattern)
        })
    }

    /// Get all edges from a node
    pub fn edges_from(&self, from: &str) -> Vec<&WiringEdge> {
        self.edges.iter().filter(|e| e.from == from).collect()
    }

    /// Get all edges to a node
    pub fn edges_to(&self, to: &str) -> Vec<&WiringEdge> {
        self.edges.iter().filter(|e| e.to == to).collect()
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        let nodes_json: Vec<String> = self.nodes.iter()
            .map(|n| format!("    {{ \"id\": \"{}\" }}", n.id))
            .collect();
        
        let edges_json: Vec<String> = self.edges.iter()
            .map(|e| format!(
                "    {{ \"from\": \"{}\", \"to\": \"{}\", \"kind\": \"{}\", \"via\": \"{}\", \"location\": \"{}\" }}",
                e.from, e.to, e.kind, e.via, e.location
            ))
            .collect();

        format!(
            "{{\n  \"nodes\": [\n{}\n  ],\n  \"edges\": [\n{}\n  ]\n}}",
            nodes_json.join(",\n"),
            edges_json.join(",\n")
        )
    }
}

/// Expected wiring specification parsed from specs/wiring-spec.aln
#[derive(Clone, Debug, Default)]
pub struct ExpectedWiring {
    pub nodes: Vec<String>,
    pub required_edges: Vec<(String, String, String)>, // (from, to, via_pattern)
    pub forbidden_edges: Vec<(String, String)>,         // (from, to)
}

/// Build wiring graph by scanning core engine modules
pub fn build_wiring_graph(vfs_content: &HashMap<String, String>) -> WiringGraph {
    let mut graph = WiringGraph::new();
    
    // Scan each core module for wiring patterns
    if let Some(lib_code) = vfs_content.get("coreengine/src/lib.rs") {
        detect_lib_wiring(lib_code, &mut graph);
    }
    
    if let Some(taskqueue_code) = vfs_content.get("coreengine/src/taskqueue.rs") {
        detect_taskqueue_wiring(taskqueue_code, &mut graph);
    }
    
    if let Some(validator_code) = vfs_content.get("coreengine/src/validator.rs") {
        detect_validator_wiring(validator_code, &mut graph);
    }
    
    if let Some(navigator_code) = vfs_content.get("coreengine/src/navigator.rs") {
        detect_navigator_wiring(navigator_code, &mut graph);
    }
    
    if let Some(vfs_code) = vfs_content.get("coreengine/src/vfs.rs") {
        detect_vfs_wiring(vfs_code, &mut graph);
    }
    
    if let Some(tokenwalker_code) = vfs_content.get("coreengine/src/tokenwalker.rs") {
        detect_tokenwalker_wiring(tokenwalker_code, &mut graph);
    }
    
    graph
}

/// Detect wiring patterns in lib.rs
fn detect_lib_wiring(code: &str, graph: &mut WiringGraph) {
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        // Lib -> TaskQueue via ccexecutetask
        if line.contains("TaskQueue::execute") || line.contains("task_queue.execute") {
            graph.add_edge("Lib", "TaskQueue", "ccexecutetask -> TaskQueue::execute", 
                          &format!("lib.rs:{}", line_num + 1));
        }
        
        // Lib -> Validator via ccvalidatecode
        if line.contains("Validator::run_validation") || line.contains("validator.run") {
            graph.add_edge("Lib", "Validator", "ccvalidatecode", 
                          &format!("lib.rs:{}", line_num + 1));
        }
        
        // Lib -> Vfs via ccinitvfs
        if line.contains("Vfs::from_json") || line.contains("Vfs::new") || line.contains("vfs.init") {
            graph.add_edge("Lib", "Vfs", "ccinitvfs", 
                          &format!("lib.rs:{}", line_num + 1));
        }
    }
}

/// Detect wiring patterns in taskqueue.rs
fn detect_taskqueue_wiring(code: &str, graph: &mut WiringGraph) {
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        // TaskQueue -> Validator
        if line.contains("Validator::run_validation") || line.contains("validator.validate") {
            graph.add_edge("TaskQueue", "Validator", "execute -> validate", 
                          &format!("taskqueue.rs:{}", line_num + 1));
        }
        
        // TaskQueue -> Vfs (read/write)
        if line.contains("Vfs::write") || line.contains("vfs.write") {
            graph.add_edge("TaskQueue", "Vfs", "write_file", 
                          &format!("taskqueue.rs:{}", line_num + 1));
        }
        
        if line.contains("Vfs::read") || line.contains("vfs.read") {
            graph.add_edge("TaskQueue", "Vfs", "read_file", 
                          &format!("taskqueue.rs:{}", line_num + 1));
        }
    }
}

/// Detect wiring patterns in validator.rs
fn detect_validator_wiring(code: &str, graph: &mut WiringGraph) {
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        // Validator -> TokenWalker
        if line.contains("TokenWalker::scan") || line.contains("tokenwalker.scan") || 
           line.contains("cc-token-walker") {
            graph.add_edge("Validator", "TokenWalker", "run_validation -> scan", 
                          &format!("validator.rs:{}", line_num + 1));
        }
    }
}

/// Detect wiring patterns in navigator.rs
fn detect_navigator_wiring(code: &str, graph: &mut WiringGraph) {
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        // Navigator -> Vfs (list/traverse)
        if line.contains("Vfs::list") || line.contains("vfs.list") || 
           line.contains("vfs.read_dir") {
            graph.add_edge("Navigator", "Vfs", "list/traverse", 
                          &format!("navigator.rs:{}", line_num + 1));
        }
    }
}

/// Detect wiring patterns in vfs.rs
fn detect_vfs_wiring(code: &str, graph: &mut WiringGraph) {
    // Vfs is typically a leaf node, but we can check for external calls
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        if line.contains("GitHubAdapter") || line.contains("github.fetch") {
            graph.add_edge("Vfs", "GitHubAdapter", "fetchRepo", 
                          &format!("vfs.rs:{}", line_num + 1));
        }
    }
}

/// Detect wiring patterns in tokenwalker.rs
fn detect_tokenwalker_wiring(code: &str, graph: &mut WiringGraph) {
    // TokenWalker is typically a leaf node (no outgoing calls to core modules)
    // We just note its existence
    let _ = code; // Suppress unused warning
}

/// Parse expected wiring from specs/wiring-spec.aln
pub fn parse_wiring_spec(spec_content: &str) -> ExpectedWiring {
    let mut expected = ExpectedWiring::default();
    
    let mut in_nodes = false;
    let mut in_edges = false;
    let mut in_forbidden = false;
    
    for line in spec_content.lines() {
        let trimmed = line.trim();
        
        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("--") {
            continue;
        }
        
        // Detect sections
        if trimmed.contains("nodes") {
            in_nodes = true;
            in_edges = false;
            in_forbidden = false;
            continue;
        }
        
        if trimmed.contains("edges") {
            in_nodes = false;
            in_edges = true;
            in_forbidden = false;
            continue;
        }
        
        if trimmed.contains("forbidden") {
            in_nodes = false;
            in_edges = false;
            in_forbidden = true;
            continue;
        }
        
        // Parse content based on current section
        if in_nodes {
            // Parse node list: "nodes Lib, Validator, Vfs" or individual lines
            let node_part = trimmed.replace("nodes", "").replace(",", " ");
            for node in node_part.split_whitespace() {
                let node = node.trim().trim_matches(',');
                if !node.is_empty() && !expected.nodes.contains(&node.to_string()) {
                    expected.nodes.push(node.to_string());
                }
            }
        } else if in_edges {
            // Parse edge: "from Lib to TaskQueue via ccexecutetask"
            if trimmed.starts_with("from") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 6 && parts[2] == "to" && parts[4] == "via" {
                    let from = parts[1].to_string();
                    let to = parts[3].to_string();
                    let via = parts[5..].join(" ");
                    expected.required_edges.push((from, to, via));
                }
            }
        } else if in_forbidden {
            // Parse forbidden edge: "from Lib to Navigator"
            if trimmed.starts_with("from") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 && parts[2] == "to" {
                    let from = parts[1].to_string();
                    let to = parts[3].to_string();
                    expected.forbidden_edges.push((from, to));
                }
            }
        }
    }
    
    expected
}

/// Validate actual wiring against expected specification
pub struct WiringValidationResult {
    pub missing_nodes: Vec<String>,
    pub extra_nodes: Vec<String>,
    pub missing_edges: Vec<String>,
    pub forbidden_violations: Vec<String>,
    pub is_valid: bool,
}

/// Wiring telemetry for efficiency metrics (DR55)
#[derive(Clone, Debug, Default)]
pub struct WiringTelemetry {
    /// Number of cross-module calls for a validateonly task
    pub validateonly_cross_calls: u32,
    /// Expected minimum (typically 3: Lib→TaskQueue→Validator→TokenWalker)
    pub validateonly_expected_min: u32,
    /// Efficiency ratio: expected_min / actual (1.0 = ideal, <1.0 = inefficient)
    pub validateonly_efficiency: f32,
    /// Breakdown of specific hops detected
    pub has_lib_to_taskqueue: bool,
    pub has_taskqueue_to_validator: bool,
    pub has_validator_to_tokenwalker: bool,
}

impl WiringTelemetry {
    pub fn new() -> Self {
        Self {
            validateonly_expected_min: 3,
            ..Default::default()
        }
    }

    /// Compute wiring telemetry from a wiring graph
    pub fn from_graph(graph: &WiringGraph) -> Self {
        let mut telemetry = Self::new();
        
        // Check for ideal path hops
        telemetry.has_lib_to_taskqueue = graph.edges.iter().any(|e| 
            e.from == "Lib" && e.to == "TaskQueue"
        );
        
        telemetry.has_taskqueue_to_validator = graph.edges.iter().any(|e| 
            e.from == "TaskQueue" && e.to == "Validator"
        );
        
        telemetry.has_validator_to_tokenwalker = graph.edges.iter().any(|e| 
            e.from == "Validator" && e.to == "TokenWalker"
        );
        
        // Count cross-module calls for validateonly path
        let mut cross_calls = 0u32;
        if telemetry.has_lib_to_taskqueue { cross_calls += 1; }
        if telemetry.has_taskqueue_to_validator { cross_calls += 1; }
        if telemetry.has_validator_to_tokenwalker { cross_calls += 1; }
        
        telemetry.validateonly_cross_calls = cross_calls;
        
        // Compute efficiency
        if cross_calls > 0 {
            telemetry.validateonly_efficiency = telemetry.validateonly_expected_min as f32 / cross_calls as f32;
        } else {
            telemetry.validateonly_efficiency = 0.0;
        }
        
        telemetry
    }

    /// Serialize to JSON for ResearchObject embedding
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"validateonly_cross_calls":{},"validateonly_expected_min":{},"validateonly_efficiency":{:.2},"has_lib_to_taskqueue":{},"has_taskqueue_to_validator":{},"has_validator_to_tokenwalker":{}}"#,
            self.validateonly_cross_calls,
            self.validateonly_expected_min,
            self.validateonly_efficiency,
            self.has_lib_to_taskqueue,
            self.has_taskqueue_to_validator,
            self.has_validator_to_tokenwalker
        )
    }
}

pub fn check_wiring_graph(actual: &WiringGraph, expected: &ExpectedWiring) -> WiringValidationResult {
    let mut result = WiringValidationResult {
        missing_nodes: Vec::new(),
        extra_nodes: Vec::new(),
        missing_edges: Vec::new(),
        forbidden_violations: Vec::new(),
        is_valid: true,
    };
    
    // Check nodes
    for node in &expected.nodes {
        if !actual.has_node(node) {
            result.missing_nodes.push(node.clone());
            result.is_valid = false;
        }
    }
    
    for node in &actual.nodes {
        if !expected.nodes.contains(&node.id) {
            result.extra_nodes.push(node.id.clone());
        }
    }
    
    // Check required edges
    for (from, to, via_pattern) in &expected.required_edges {
        if !actual.has_edge(from, to, via_pattern) {
            result.missing_edges.push(
                format!("Missing edge: {} -> {} via '{}'", from, to, via_pattern)
            );
            result.is_valid = false;
        }
    }
    
    // Check forbidden edges
    for (from, to) in &expected.forbidden_edges {
        if actual.edges_from(from).iter().any(|e| e.to == *to) {
            let edge_locations: Vec<String> = actual.edges_from(from)
                .filter(|e| e.to == *to)
                .map(|e| format!("{} (via '{}' at {})", e.to, e.via, e.location))
                .collect();
            
            result.forbidden_violations.push(
                format!("Forbidden edge detected: {} -> {} [{}]", from, to, edge_locations.join(", "))
            );
            result.is_valid = false;
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiring_graph_basic() {
        let mut graph = WiringGraph::new();
        graph.add_edge("Lib", "TaskQueue", "ccexecutetask", "lib.rs:42");
        
        assert!(graph.has_node("Lib"));
        assert!(graph.has_node("TaskQueue"));
        assert!(graph.has_edge("Lib", "TaskQueue", "ccexecutetask"));
        assert!(!graph.has_edge("Lib", "Validator", "ccexecutetask"));
    }

    #[test]
    fn test_wiring_graph_json() {
        let mut graph = WiringGraph::new();
        graph.add_edge("Lib", "TaskQueue", "ccexecutetask", "lib.rs:42");
        
        let json = graph.to_json();
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
        assert!(json.contains("Lib"));
        assert!(json.contains("TaskQueue"));
    }

    #[test]
    fn test_parse_wiring_spec() {
        let spec = r#"
nodes Lib, Validator, Vfs

edges
  from Lib to TaskQueue via ccexecutetask
  from TaskQueue to Validator via validate

forbidden
  from Lib to Navigator
"#;
        
        let expected = parse_wiring_spec(spec);
        assert!(expected.nodes.contains(&"Lib".to_string()));
        assert_eq!(expected.required_edges.len(), 2);
        assert_eq!(expected.forbidden_edges.len(), 1);
    }
}
