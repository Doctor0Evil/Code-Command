// core/engine/src/wiring_graph.rs

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
