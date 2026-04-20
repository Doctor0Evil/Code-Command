// FILE: ./core/engine/src/wiring_validator.rs

use crate::wiring_graph::{WiringEdge, WiringGraph, WiringNode};
use crate::validator::{Severity, ValidationEntry, ValidationResult};
use crate::vfs::Vfs;

#[derive(Debug)]
pub struct WiringSpec {
    pub graph: WiringGraph,
    pub required_edges: Vec<WiringEdge>,
    pub forbidden_edges: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum WiringLoadError {
    NotFound,
    ParseError(String),
}

pub struct WiringValidator {
    pub spec: WiringSpec,
}

impl WiringValidator {
    /// Load and parse `.specs/wiring-spec.aln` from the virtual filesystem into a WiringSpec.
    pub fn from_aln(vfs: &Vfs) -> Result<Self, WiringLoadError> {
        let path = ".specs/wiring-spec.aln";
        let text = vfs
            .read(path)
            .ok_or(WiringLoadError::NotFound)?;
        let spec = parse_wiring_spec_from_aln(&text)?;
        Ok(WiringValidator { spec })
    }

    /// Validate the actual WiringGraph against the spec:
    /// - all expected nodes exist,
    /// - each required edge is present (matching via),
    /// - no forbidden edge exists.
    /// Failures are tagged `CC-WIRING`.
    pub fn validate(&self, actual: &WiringGraph) -> ValidationResult {
        let mut entries: Vec<ValidationEntry> = Vec::new();

        // Node set for quick lookup
        let actual_nodes: std::collections::HashSet<&str> =
            actual.nodes.iter().map(|n| n.id.as_str()).collect();

        // 1. Node check
        for expected in &self.spec.graph.nodes {
            if !actual_nodes.contains(expected.id.as_str()) {
                entries.push(ValidationEntry {
                    tag: "CC-WIRING".to_string(),
                    passed: false,
                    message: format!("Missing node `{}` required by wiring-spec.", expected.id),
                    path: ".specs/wiring-spec.aln".to_string(),
                    line: 0,
                    column: 0,
                    severity: Severity::Error,
                });
            }
        }

        // 2. Required edge check
        for required in &self.spec.required_edges {
            let has_match = actual.edges.iter().any(|e| {
                e.from == required.from && e.to == required.to && e.via == required.via
            });

            if !has_match {
                entries.push(ValidationEntry {
                    tag: "CC-WIRING".to_string(),
                    passed: false,
                    message: format!(
                        "Missing required wiring edge `{}` -> `{}` via `{}`.",
                        required.from, required.to, required.via
                    ),
                    path: ".specs/wiring-spec.aln".to_string(),
                    line: 0,
                    column: 0,
                    severity: Severity::Error,
                });
            }
        }

        // Build a set of forbidden (from,to) pairs for fast check
        let forbidden: std::collections::HashSet<(String, String)> = self
            .spec
            .forbidden_edges
            .iter()
            .cloned()
            .collect();

        // 3. Forbidden edge check
        for edge in &actual.edges {
            if forbidden.contains(&(edge.from.clone(), edge.to.clone())) {
                entries.push(ValidationEntry {
                    tag: "CC-WIRING".to_string(),
                    passed: false,
                    message: format!(
                        "Forbidden wiring edge `{}` -> `{}` present but disallowed by wiring-spec.",
                        edge.from, edge.to
                    ),
                    path: ".specs/wiring-spec.aln".to_string(),
                    line: 0,
                    column: 0,
                    severity: Severity::Error,
                });
            }
        }

        let ok = entries.iter().all(|e| e.passed);
        ValidationResult { ok, entries }
    }
}

/// Minimal ALN parser for `specs/wiring-spec.aln`.
/// Expects sections:
/// - `nodes` with `id`
/// - `edges` with `from`, `to`, `via`
/// - `invariants` with `requirededge` and `forbiddenedge`.
fn parse_wiring_spec_from_aln(src: &str) -> Result<WiringSpec, WiringLoadError> {
    enum Section {
        None,
        Nodes,
        Edges,
        Invariants,
    }

    let mut section = Section::None;
    let mut nodes: Vec<WiringNode> = Vec::new();
    let mut edges: Vec<WiringEdge> = Vec::new();
    let mut required_edges: Vec<WiringEdge> = Vec::new();
    let mut forbidden_edges: Vec<(String, String)> = Vec::new();

    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("!--") {
            continue;
        }

        // Section switches
        if line.starts_with("nodes") {
            section = Section::Nodes;
            continue;
        }
        if line.starts_with("edges") {
            section = Section::Edges;
            continue;
        }
        if line.starts_with("invariants") {
            section = Section::Invariants;
            continue;
        }

        match section {
            Section::Nodes => {
                if line.starts_with("-") {
                    // Format: - id Lib type core
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut id: Option<String> = None;
                    for w in parts.windows(2) {
                        if w[0] == "id" {
                            id = Some(w[1].trim_matches(',').to_string());
                        }
                    }
                    if let Some(id) = id {
                        nodes.push(WiringNode { id });
                    }
                }
            }
            Section::Edges => {
                if line.starts_with("-") {
                    // Format: - from Lib to TaskQueue via ccexecutetask
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut from: Option<String> = None;
                    let mut to: Option<String> = None;
                    let mut via: Option<String> = None;
                    let mut i = 0;
                    while i + 1 < parts.len() {
                        match parts[i] {
                            "from" => {
                                from = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            "to" => {
                                to = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            "via" => {
                                via = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            _ => i += 1,
                        }
                    }
                    if let (Some(from), Some(to), Some(via)) = (from, to, via) {
                        edges.push(WiringEdge {
                            from,
                            to,
                            kind: "call".to_string(),
                            via,
                        });
                    }
                }
            }
            Section::Invariants => {
                if line.contains("requirededge") {
                    // Next lines (or same line) contain from, to, via
                    // We reuse the same simple splitter, expecting the ALN
                    // to be in the documented format.
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut from: Option<String> = None;
                    let mut to: Option<String> = None;
                    let mut via: Option<String> = None;
                    let mut i = 0;
                    while i + 1 < parts.len() {
                        match parts[i] {
                            "from" => {
                                from = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            "to" => {
                                to = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            "via" => {
                                via = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            _ => i += 1,
                        }
                    }
                    if let (Some(from), Some(to), Some(via)) = (from, to, via) {
                        required_edges.push(WiringEdge { from, to, kind: "call".to_string(), via });
                    }
                } else if line.contains("forbiddenedge") {
                    // Format: forbiddenedge from TaskQueue to Navigator
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut from: Option<String> = None;
                    let mut to: Option<String> = None;
                    let mut i = 0;
                    while i + 1 < parts.len() {
                        match parts[i] {
                            "from" => {
                                from = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            "to" => {
                                to = Some(parts[i + 1].trim_matches(',').to_string());
                                i += 2;
                            }
                            _ => i += 1,
                        }
                    }
                    if let (Some(from), Some(to)) = (from, to) {
                        forbidden_edges.push((from, to));
                    }
                }
            }
            Section::None => {}
        }
    }

    if nodes.is_empty() {
        return Err(WiringLoadError::ParseError(
            "No nodes found in wiring-spec.aln".to_string(),
        ));
    }

    let graph = WiringGraph { nodes, edges };
    Ok(WiringSpec {
        graph,
        required_edges,
        forbidden_edges,
    })
}
