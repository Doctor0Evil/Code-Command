// FILE: ./core/engine/src/capacity_specs.rs

use std::collections::HashMap;

/// Per-class placement entry: routing fractions r_{k,j} and policy knobs.
#[derive(Clone, Debug)]
pub struct PlacementTierFraction {
    pub tier_id: String,
    pub fraction: f64,
}

#[derive(Clone, Debug)]
pub struct PlacementClassEntry {
    pub sko_class_id: String,
    pub utility_weight: f64,
    pub latency_sla_seconds: f64,
    pub tiers: Vec<PlacementTierFraction>,
}

/// Full placement plan model.
#[derive(Clone, Debug)]
pub struct PlacementPlan {
    pub horizon_hours: f64,
    pub roh_ceiling: f64,
    pub entries: Vec<PlacementClassEntry>,
}

/// SKO size distribution kind.
#[derive(Clone, Debug)]
pub enum SizeDistKind {
    LogNormal,
    Point,
}

/// SKO size distribution parameters.
#[derive(Clone, Debug)]
pub struct SizeDistribution {
    pub kind: SizeDistKind,
    pub mean_bytes: f64,
    pub stddev_bytes: f64,
}

/// Placement preferences r_{k,j} as originally carried on SKO classes.
/// (You can keep using this for the legacy sko-size-model spec, while
/// PlacementPlan is the newer, explicit placement schema.)
#[derive(Clone, Debug)]
pub struct PlacementPreference {
    pub tier_id: String,
    pub fraction: f64,
}

/// One SKO class entry from sko-size-model.
#[derive(Clone, Debug)]
pub struct SkoClass {
    pub id: String,
    pub label: String,
    pub arrival_rate_lambda: f64, // SKOs per hour
    pub size_dist: SizeDistribution,
    pub placements: Vec<PlacementPreference>,
}

/// Top-level SKO size model spec.
#[derive(Clone, Debug)]
pub struct SkoSizeModel {
    pub horizon_hours: f64,
    pub classes: Vec<SkoClass>,
    pub default_size_dist: SizeDistribution,
}

/// Per-tier eco parameters.
#[derive(Clone, Debug)]
pub struct TierEco {
    pub pue: f64,
    pub power_idle_W_per_TB: f64,
    pub power_active_W_per_TB: f64,
    pub energy_budget_kWh: f64,
    pub carbon_budget_kgCO2e: f64,
    pub grid_carbon_intensity_kg_per_kWh: f64,
}

/// Per-tier capacity and queueing parameters.
#[derive(Clone, Debug)]
pub struct TierCapacity {
    pub id: String,
    pub label: String,
    pub capacity_TiB: f64,
    pub target_utilization_rho: f64,
    pub max_utilization_rho: f64,
    pub service_rate_mu: f64, // SKOs per second
    pub eco: TierEco,
}

/// Top-level tier capacity spec.
#[derive(Clone, Debug)]
pub struct TierCapacityModel {
    pub tiers: Vec<TierCapacity>,
}

/// Per-tier computed metrics, aligned with CapacityReport.
#[derive(Clone, Debug)]
pub struct TierReport {
    pub tier_id: String,
    pub expected_storage_TiB: f64,
    pub capacity_TiB: f64,
    pub target_utilization_rho: f64,
    pub expected_utilization_rho: f64,
    pub service_rate_mu: f64,
    pub aggregate_arrival_rate_lambda: f64,
    pub rho_queue: f64,
    pub expected_wait_seconds: f64,
    pub energy_used_kWh: f64,
    pub carbon_used_kgCO2e: f64,
    pub within_storage_envelope: bool,
    pub within_eco_envelope: bool,
    pub within_latency_envelope: bool,
}

/// Logical capacity and eco-wealth summary.
#[derive(Clone, Debug)]
pub struct LogicalCapacitySummary {
    pub n_logical_max: f64,
    pub kappa_ratio: f64,
    pub class_proportions: HashMap<String, f64>,
    pub ew_numerator: f64,
    pub ew_denominator: f64,
    pub ew_score: f64,
    pub roh_score: f64,
    pub roh_ceiling: f64,
    pub within_roh_ceiling: bool,
}

/// Full capacity report to serialize as JSON or ALN-compatible output.
#[derive(Clone, Debug)]
pub struct CapacityReport {
    pub horizon_hours: f64,
    pub tier_reports: Vec<TierReport>,
    pub logical_summary: LogicalCapacitySummary,
}
