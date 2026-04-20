// FILE: ./core/engine/src/wiring_graph.rs

#[derive(Clone, Debug)]
pub struct WiringNode {
    pub id: String,   // e.g. "Lib", "Validator", "Vfs", "Navigator", "TaskQueue"
}

#[derive(Clone, Debug)]
pub struct WiringEdge {
    pub from: String, // node id
    pub to: String,   // node id
    pub kind: String, // "call"
    pub via: String,  // function name, e.g. "Lib::ccexecutetask -> TaskQueue::execute"
}

#[derive(Clone, Debug)]
pub struct WiringGraph {
    pub nodes: Vec<WiringNode>,
    pub edges: Vec<WiringEdge>,
}

impl WiringGraph {
    /// Serialize the wiring graph to compact JSON for external tools.
    /// Format: {"nodes":[{"id":"..."},...],"edges":[{"from":"...","to":"...","kind":"...","via":"..."},...]}
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        
        // Nodes array
        out.push_str("\"nodes\":[");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"id\":\"");
            out.push_str(&escape_json(&node.id));
            out.push('\"');
            out.push('}');
        }
        out.push(']');
        
        // Edges array
        out.push_str(",\"edges\":[");
        for (i, edge) in self.edges.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"from\":\"");
            out.push_str(&escape_json(&edge.from));
            out.push_str("\",\"to\":\"");
            out.push_str(&escape_json(&edge.to));
            out.push_str("\",\"kind\":\"");
            out.push_str(&escape_json(&edge.kind));
            out.push_str("\",\"via\":\"");
            out.push_str(&escape_json(&edge.via));
            out.push('\"');
            out.push('}');
        }
        out.push(']');
        
        out.push('}');
        out
    }
}

fn escape_json(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
