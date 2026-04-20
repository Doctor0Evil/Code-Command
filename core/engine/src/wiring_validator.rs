// FILE: ./core/engine/src/wiring_validator.rs

use crate::wiring_graph::WiringGraph;
use crate::validator::{ValidationResult, ValidationEntry, Severity};

pub struct WiringSpec {
    pub graph: WiringGraph, // expected nodes/edges loaded from ALN
}

pub struct WiringValidator {
    pub spec: WiringSpec,
}

impl WiringValidator {
    pub fn from_aln(spec_src: &str) -> Result<Self, WiringLoadError>;

    pub fn validate(&self, actual: &WiringGraph) -> ValidationResult;
}
