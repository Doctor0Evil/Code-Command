// datalake/core/src/ecoplacementv2.rs

use crate::tagsolver::{LegalTagVector, TagCompatibility};
use crate::eco_wealth::EcoWealthParams;
use crate::queue_model::WorkloadProfile;

#[derive(Clone, Debug)]
pub struct HardwareTierId(pub String);

#[derive(Clone, Debug)]
pub struct MediaId(pub String);

#[derive(Clone, Debug)]
pub struct VaultCandidate {
    pub media: MediaId,
    pub tier: HardwareTierId,
}

#[derive(Clone, Debug)]
pub struct CostVector {
    pub energy_kwh: f64,      // E
    pub carbon_kg: f64,       // C
    pub latency_ms: f64,      // L
    pub roh_score: f64,       // R in [0,1]
}

#[derive(Clone, Debug)]
pub struct PlacementCandidate {
    pub vault: VaultCandidate,
    pub costs: CostVector,
}

#[derive(Clone, Debug)]
pub struct PlacementDecision {
    pub vault: VaultCandidate,
    pub costs: CostVector,
    pub rationale: String,
}

#[derive(Clone, Debug)]
pub enum PreferenceMode {
    WeightedSum,
    Lexicographic,
}

#[derive(Clone, Debug)]
pub struct PreferenceWeights {
    pub w_energy: f64,
    pub w_carbon: f64,
    pub w_latency: f64,
    pub w_roh: f64,
    pub mode: PreferenceMode,
}

#[derive(Clone, Debug)]
pub struct JurisdictionProfile {
    pub id: String,
    pub roh_max: f64,
    pub preference: PreferenceWeights,
}

#[derive(thiserror::Error, Debug)]
pub enum EcoPlacementError {
    #[error("no feasible candidates after constraints")]
    NoFeasibleCandidates,
    #[error("tag compatibility error: {0}")]
    TagError(String),
}

pub struct HardwareCatalog; // placeholder – wire to YAML-backed catalog

impl HardwareCatalog {
    pub fn feasible_candidates(
        &self,
        tags: &LegalTagVector,
        workload: &WorkloadProfile,
        jurisdiction: &JurisdictionProfile,
        tag_solver: &dyn TagCompatibility,
    ) -> Result<Vec<PlacementCandidate>, EcoPlacementError> {
        // 1. Enumerate all (media, tier) pairs from catalog.
        // 2. Filter by tag/jurisdiction compatibility.
        // 3. Compute RoH; drop any with roh_score > roh_max.
        // 4. Compute energy, carbon, latency costs for workload.
        // 5. Return list of candidates.
        let _ = (tags, workload, jurisdiction, tag_solver); // silence warnings
        Ok(Vec::new())
    }
}

fn dominates(a: &CostVector, b: &CostVector) -> bool {
    let le_all =
        a.energy_kwh <= b.energy_kwh &&
        a.carbon_kg <= b.carbon_kg &&
        a.latency_ms <= b.latency_ms &&
        a.roh_score <= b.roh_score;

    let lt_any =
        a.energy_kwh < b.energy_kwh ||
        a.carbon_kg < b.carbon_kg ||
        a.latency_ms < b.latency_ms ||
        a.roh_score < b.roh_score;

    le_all && lt_any
}

fn pareto_frontier(candidates: &[PlacementCandidate]) -> Vec<PlacementCandidate> {
    let mut frontier = Vec::new();
    'outer: for c in candidates {
        for other in candidates {
            if dominates(&other.costs, &c.costs) {
                continue 'outer;
            }
        }
        frontier.push(c.clone());
    }
    frontier
}

fn score_weighted(c: &PlacementCandidate, w: &PreferenceWeights) -> f64 {
    w.w_energy * c.costs.energy_kwh
        + w.w_carbon * c.costs.carbon_kg
        + w.w_latency * c.costs.latency_ms
        + w.w_roh * c.costs.roh_score
}

fn select_from_frontier(
    frontier: Vec<PlacementCandidate>,
    pref: &PreferenceWeights,
) -> Result<PlacementCandidate, EcoPlacementError> {
    if frontier.is_empty() {
        return Err(EcoPlacementError::NoFeasibleCandidates);
    }

    match pref.mode {
        PreferenceMode::WeightedSum => {
            let mut best = &frontier[0];
            let mut best_score = score_weighted(best, pref);
            for c in &frontier[1..] {
                let s = score_weighted(c, pref);
                if s < best_score {
                    best = c;
                    best_score = s;
                }
            }
            Ok(best.clone())
        }
        PreferenceMode::Lexicographic => {
            let mut sorted = frontier;
            sorted.sort_by(|a, b| {
                a.costs
                    .carbon_kg
                    .partial_cmp(&b.costs.carbon_kg)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        a.costs
                            .energy_kwh
                            .partial_cmp(&b.costs.energy_kwh)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        a.costs
                            .roh_score
                            .partial_cmp(&b.costs.roh_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        a.costs
                            .latency_ms
                            .partial_cmp(&b.costs.latency_ms)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            Ok(sorted[0].clone())
        }
    }
}

pub fn decide_placement_v2(
    tags: &LegalTagVector,
    workload: &WorkloadProfile,
    jurisdiction: &JurisdictionProfile,
    catalog: &HardwareCatalog,
    tag_solver: &dyn TagCompatibility,
    eco_params: &EcoWealthParams,
) -> Result<PlacementDecision, EcoPlacementError> {
    let _ = eco_params; // reserved for later eco-wealth integration

    let candidates = catalog.feasible_candidates(tags, workload, jurisdiction, tag_solver)?;
    if candidates.is_empty() {
        return Err(EcoPlacementError::NoFeasibleCandidates);
    }

    let frontier = pareto_frontier(&candidates);
    let chosen = select_from_frontier(frontier, &jurisdiction.preference)?;

    let rationale = format!(
        "selected media={} tier={} E={:.3}kWh C={:.3}kg L={:.2}ms R={:.3}",
        chosen.vault.media.0,
        chosen.vault.tier.0,
        chosen.costs.energy_kwh,
        chosen.costs.carbon_kg,
        chosen.costs.latency_ms,
        chosen.costs.roh_score
    );

    Ok(PlacementDecision {
        vault: chosen.vault.clone(),
        costs: chosen.costs.clone(),
        rationale,
    })
}
