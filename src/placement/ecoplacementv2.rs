// FILE: ./src/placement/ecoplacementv2.rs
// CC-LANG: rs
// CC-FILE: src/placement/ecoplacementv2.rs
// CC-DEEP: ./src/placement/ecoplacementv2.rs

use crate::catalog::grid_carbon_catalog::{GridCarbonCatalog, SimpleTimestamp};
use crate::catalog::hardware_power::HardwarePowerCatalog;
use crate::catalog::embodied_carbon::{EmbodiedCatalog, EmbodiedProfile};
use crate::risk::risk_model::{RiskModel, RohInputs};
use crate::workload::sko_workload::WorkloadRegistry;

#[derive(Clone, Debug)]
pub struct EcoPlacementContext<'a> {
    pub grid_carbon: &'a dyn GridCarbonCatalog,
    pub hw_power: &'a dyn HardwarePowerCatalog,
    pub embodied: &'a EmbodiedCatalog,
    pub risk_model: &'a RiskModel,
    pub workload: &'a WorkloadRegistry,
    pub now: SimpleTimestamp,
}

#[derive(Clone, Debug)]
pub struct CandidatePlacement {
    pub vault_id: String,
    pub tier_id: String,
    pub region_id: String,
    pub generation_id: String,
    pub capacity_tb: f64,
    pub utilization: f64,
}

#[derive(Clone, Debug)]
pub struct EcoCost {
    pub energy_kwh: f64,
    pub operational_kg_co2e: f64,
    pub embodied_kg_co2e: f64,
    pub latency_ms_p95: f64,
    pub roh: f64,
}

impl<'a> EcoPlacementContext<'a> {
    pub fn estimate_cost_for_sko(
        &self,
        sko_class: &str,
        sko_sensitivity: f64,
        candidate: &CandidatePlacement,
    ) -> Option<EcoCost> {
        let vault_profile = self
            .risk_model
            .vault_profiles
            .iter()
            .find(|v| v.vault_id == candidate.vault_id)?;

        let workload_pattern_score = self
            .workload
            .expected_arrival_rate(&candidate.region_id, sko_class)
            .unwrap_or(0.1)
            .min(1.0);

        let roh_inputs = RohInputs {
            sko_class,
            sko_sensitivity,
            vault: vault_profile,
            workload_pattern_score,
        };

        if !self.risk_model.is_allowed(&roh_inputs) {
            return None;
        }

        let grid = match self
            .grid_carbon
            .get_latest_before(&candidate.region_id, self.now)
        {
            Ok(v) => v,
            Err(_) => return None,
        };

        let tier = match self.hw_power.get_tier(&candidate.tier_id) {
            Some(t) => t,
            None => return None,
        };

        let power_w = tier.power_w(candidate.utilization, candidate.capacity_tb);
        let hours = 1.0;
        let energy_kwh = power_w * hours / 1000.0;
        let operational_kg_co2e = energy_kwh * grid.carbon_g_per_kwh / 1000.0;

        let embodied_profile: EmbodiedProfile = match self
            .embodied
            .find_by_generation(&candidate.generation_id)
        {
            Some(p) => p,
            None => {
                EmbodiedProfile {
                    generation_id: candidate.generation_id.clone(),
                    tier_id: candidate.tier_id.clone(),
                    embodied_kg_co2e_per_tb: 100.0,
                    design_lifetime_years: 5.0,
                }
            }
        };

        let logical_tb_years = (candidate.capacity_tb / 100.0) * (1.0 / 365.0);
        let embodied_kg = embodied_profile.per_request_kg_co2e(logical_tb_years);

        let latency_ms_p95 = match candidate.tier_id.as_str() {
            "nvme_hot" => 5.0,
            "ssd_warm" => 20.0,
            "hdd_cold" => 80.0,
            "tape_archive" => 5000.0,
            _ => 100.0,
        };

        let roh = self.risk_model.compute_roh(&roh_inputs);

        Some(EcoCost {
            energy_kwh,
            operational_kg_co2e,
            embodied_kg_co2e: embodied_kg,
            latency_ms_p95,
            roh,
        })
    }
}
