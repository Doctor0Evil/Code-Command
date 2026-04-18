// FILE: ./src/risk/risk_model.rs
// CC-LANG: rs
// CC-FILE: src/risk/risk_model.rs
// CC-DEEP: ./src/risk/risk_model.rs

#[derive(Clone, Debug)]
pub struct VaultRiskProfile {
    pub vault_id: String,
    pub jurisdiction: String,
    pub baseline_risk_score: f64,
    pub physical_security_score: f64,
    pub network_exposure_score: f64,
    pub operator_trust_score: f64,
    pub max_allowed_roh_for_neural: f64,
    pub max_allowed_roh_for_standard: f64,
}

pub struct RiskModel {
    pub vault_profiles: Vec<VaultRiskProfile>,
}

#[derive(Clone, Debug)]
pub struct RohInputs<'a> {
    pub sko_class: &'a str,
    pub sko_sensitivity: f64,
    pub vault: &'a VaultRiskProfile,
    pub workload_pattern_score: f64,
}

impl RiskModel {
    pub fn new(vault_profiles: Vec<VaultRiskProfile>) -> Self {
        Self { vault_profiles }
    }

    pub fn compute_roh(&self, inputs: &RohInputs<'_>) -> f64 {
        let base = inputs.vault.baseline_risk_score;
        let physical = (1.0 - inputs.vault.physical_security_score) * 0.3;
        let network = inputs.vault.network_exposure_score * 0.4;
        let operator = (1.0 - inputs.vault.operator_trust_score) * 0.3;
        let workload = inputs.workload_pattern_score * 0.5;
        let sensitivity = inputs.sko_sensitivity;

        let raw = base + physical + network + operator + workload;
        let scored = raw * sensitivity;
        if scored < 0.0 {
            0.0
        } else if scored > 1.0 {
            1.0
        } else {
            scored
        }
    }

    pub fn is_allowed(&self, inputs: &RohInputs<'_>) -> bool {
        let roh = self.compute_roh(inputs);
        let limit = if inputs.sko_class == "neural" {
            inputs.vault.max_allowed_roh_for_neural
        } else {
            inputs.vault.max_allowed_roh_for_standard
        };
        roh <= limit
    }
}
