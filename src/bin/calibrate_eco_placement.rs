// FILE: ./src/bin/calibrate_eco_placement.rs
// CC-LANG: rs
// CC-FILE: src/bin/calibrate_eco_placement.rs
// CC-DEEP: ./src/bin/calibrate_eco_placement.rs

use code_command_engine::catalog::grid_carbon_catalog::{
    GridSample, InMemoryGridCarbonCatalog, RegionTimeSeries, SimpleTimestamp,
};
use code_command_engine::catalog::hardware_power::{
    InMemoryHardwarePowerCatalog, TierPowerProfile,
};
use code_command_engine::catalog::embodied_carbon::{
    EmbodiedCatalog, EmbodiedProfile,
};
use code_command_engine::risk::risk_model::{RiskModel, VaultRiskProfile};
use code_command_engine::workload::sko_workload::{
    RegionWorkloadDistribution, SkoClassDescriptor, WorkloadRegistry,
};
use code_command_engine::placement::ecoplacementv2::{
    CandidatePlacement, EcoPlacementContext,
};

fn main() {
    let ts0 = SimpleTimestamp::new(1_771_382_400);
    let mut samples = std::collections::BTreeMap::new();
    samples.insert(
        ts0,
        GridSample {
            ts: ts0,
            carbon_g_per_kwh: 410.0,
            renewable_fraction: 0.24,
        },
    );
    let phx_series = RegionTimeSeries {
        region_id: "us-west-phx".to_string(),
        samples,
    };
    let grid_catalog = InMemoryGridCarbonCatalog::new(vec![phx_series]);

    let hw_catalog = InMemoryHardwarePowerCatalog::new(vec![
        TierPowerProfile {
            tier_id: "nvme_hot".to_string(),
            p_idle_w_per_tb: 0.40,
            p_active_w_per_tb: 2.50,
            pue: 1.18,
        },
        TierPowerProfile {
            tier_id: "ssd_warm".to_string(),
            p_idle_w_per_tb: 0.80,
            p_active_w_per_tb: 3.00,
            pue: 1.26,
        },
    ]);

    let embodied_catalog = EmbodiedCatalog::new(vec![EmbodiedProfile {
        generation_id: "nvme_2025_gen4".to_string(),
        tier_id: "nvme_hot".to_string(),
        embodied_kg_co2e_per_tb: 180.0,
        design_lifetime_years: 5.0,
    }]);

    let risk_model = RiskModel::new(vec![VaultRiskProfile {
        vault_id: "phx-vault-a".to_string(),
        jurisdiction: "us-az".to_string(),
        baseline_risk_score: 0.12,
        physical_security_score: 0.90,
        network_exposure_score: 0.35,
        operator_trust_score: 0.88,
        max_allowed_roh_for_neural: 0.25,
        max_allowed_roh_for_standard: 0.35,
    }]);

    let workload_registry = WorkloadRegistry {
        classes: vec![SkoClassDescriptor {
            class_id: "smart_city_telemetry".to_string(),
            sensitivity: 0.4,
            typical_size_mb: 5.0,
            typical_read_ratio: 0.60,
            typical_write_ratio: 0.80,
            target_latency_ms_p95: 250,
            deferral_window_minutes: 30,
        }],
        region_distributions: vec![RegionWorkloadDistribution {
            region_id: "us-west-phx".to_string(),
            total_skos: 10_000_000,
            fractions: vec![(
                "smart_city_telemetry".to_string(),
                0.25_f64,
            )],
        }],
    };

    let ctx = EcoPlacementContext {
        grid_carbon: &grid_catalog,
        hw_power: &hw_catalog,
        embodied: &embodied_catalog,
        risk_model: &risk_model,
        workload: &workload_registry,
        now: ts0,
    };

    let candidate = CandidatePlacement {
        vault_id: "phx-vault-a".to_string(),
        tier_id: "nvme_hot".to_string(),
        region_id: "us-west-phx".to_string(),
        generation_id: "nvme_2025_gen4".to_string(),
        capacity_tb: 100.0,
        utilization: 0.4,
    };

    if let Some(cost) = ctx.estimate_cost_for_sko(
        "smart_city_telemetry",
        0.4,
        &candidate,
    ) {
        println!("Energy_kWh: {}", cost.energy_kwh);
        println!("Operational_kg_CO2e: {}", cost.operational_kg_co2e);
        println!("Embodied_kg_CO2e: {}", cost.embodied_kg_co2e);
        println!("Latency_ms_p95: {}", cost.latency_ms_p95);
        println!("RoH: {}", cost.roh);
    } else {
        println!("Candidate rejected by RoH or missing data");
    }
}
