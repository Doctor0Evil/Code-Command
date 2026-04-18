// FILE: ./core/engine/src/capacity_engine.rs

use std::collections::HashMap;

use crate::capacity_specs::{
    CapacityReport,
    LogicalCapacitySummary,
    PlacementPlan,
    SkoSizeModel,
    TierCapacityModel,
    TierReport,
};

/// Compute expected per-tier loads and aggregate metrics over the horizon.
pub fn compute_capacity_report(
    sko_model: &SkoSizeModel,
    tier_model: &TierCapacityModel,
    placement: &PlacementPlan,
) -> CapacityReport {
    let horizon_hours = sko_model.horizon_hours.min(placement.horizon_hours);
    let horizon_seconds = horizon_hours * 3600.0;

    // Index tier capacities by id for quick lookup.
    let mut tier_map: HashMap<String, &crate::capacity_specs::TierCapacity> = HashMap::new();
    for t in &tier_model.tiers {
        tier_map.insert(t.id.clone(), t);
    }

    // Map SKO class id -> (lambda_k, E[S|k]).
    let mut class_lambda: HashMap<String, f64> = HashMap::new();
    let mut class_mean_size: HashMap<String, f64> = HashMap::new();
    for c in &sko_model.classes {
        class_lambda.insert(c.id.clone(), c.arrival_rate_lambda);
        class_mean_size.insert(c.id.clone(), c.size_dist.mean_bytes);
    }

    // Aggregate per-tier stats.
    let mut tier_storage_bytes: HashMap<String, f64> = HashMap::new();
    let mut tier_arrival_rate: HashMap<String, f64> = HashMap::new();
    let mut tier_energy_kwh: HashMap<String, f64> = HashMap::new();
    let mut tier_carbon_kg: HashMap<String, f64> = HashMap::new();

    // Logical capacity / eco-wealth intermediates.
    let mut total_physical_bytes: f64 = 0.0;
    let mut total_expected_bytes: f64 = 0.0;
    let mut class_total_lambda: f64 = 0.0;
    let mut class_weighted_utility: f64 = 0.0;

    for entry in &placement.entries {
        let lambda_k = *class_lambda
            .get(&entry.sko_class_id)
            .unwrap_or(&0.0); // SKOs/hour
        let mean_size = *class_mean_size
            .get(&entry.sko_class_id)
            .unwrap_or(&sko_model.default_size_dist.mean_bytes); // bytes

        class_total_lambda += lambda_k;
        class_weighted_utility += lambda_k * entry.utility_weight;

        for frac in &entry.tiers {
            if frac.fraction <= 0.0 {
                continue;
            }
            if let Some(tier) = tier_map.get(&frac.tier_id) {
                // Lambda_{k,j} (SKOs/hour).
                let lambda_kj = lambda_k * frac.fraction;
                // Expected storage contribution over horizon: lambda * T * E[S|k].
                let storage_bytes = lambda_kj * horizon_hours * mean_size;

                *tier_storage_bytes.entry(tier.id.clone()).or_insert(0.0) += storage_bytes;
                *tier_arrival_rate.entry(tier.id.clone()).or_insert(0.0) += lambda_kj / 3600.0; // SKOs/s

                // Approximate utilization for energy calc: u_ij ~ lambda_kj / mu_j.
                let lambda_kj_sec = lambda_kj / 3600.0;
                let u_ij = if tier.service_rate_mu > 0.0 {
                    (lambda_kj_sec / tier.service_rate_mu).min(1.0)
                } else {
                    0.0
                };

                let (e_kwh, c_kg) = estimate_energy_and_carbon(
                    tier,
                    u_ij,
                    horizon_hours,
                );

                *tier_energy_kwh.entry(tier.id.clone()).or_insert(0.0) += e_kwh;
                *tier_carbon_kg.entry(tier.id.clone()).or_insert(0.0) += c_kg;
            }
        }

        total_expected_bytes += lambda_k * horizon_hours * mean_size;
    }

    // Build per-tier reports.
    let mut tier_reports: Vec<TierReport> = Vec::new();

    for tier in &tier_model.tiers {
        let storage_bytes = *tier_storage_bytes.get(&tier.id).unwrap_or(&0.0);
        let storage_tib = storage_bytes / (1024.0_f64.powi(4));

        let capacity_tib = tier.capacity_TiB;
        let expected_util = if capacity_tib > 0.0 {
            (storage_tib / capacity_tib).min(1.0)
        } else {
            0.0
        };

        let lambda_j = *tier_arrival_rate.get(&tier.id).unwrap_or(&0.0); // SKOs/s
        let rho_queue = if tier.service_rate_mu > 0.0 {
            (lambda_j / tier.service_rate_mu).min(0.999_999)
        } else {
            0.0
        };
        let expected_wait = if tier.service_rate_mu > 0.0 && rho_queue < 1.0 {
            rho_queue / (tier.service_rate_mu * (1.0 - rho_queue))
        } else {
            f64::INFINITY
        };

        let energy_kwh = *tier_energy_kwh.get(&tier.id).unwrap_or(&0.0);
        let carbon_kg = *tier_carbon_kg.get(&tier.id).unwrap_or(&0.0);

        let within_storage = expected_util <= tier.target_utilization_rho;
        let within_eco = energy_kwh <= tier.eco.energy_budget_kWh
            && carbon_kg <= tier.eco.carbon_budget_kgCO2e;
        let within_latency =
            expected_wait <= derive_latency_sla_for_tier(&tier.id, &placement);

        total_physical_bytes += capacity_tib * 1024.0_f64.powi(4);

        tier_reports.push(TierReport {
            tier_id: tier.id.clone(),
            expected_storage_TiB: storage_tib,
            capacity_TiB: capacity_tib,
            target_utilization_rho: tier.target_utilization_rho,
            expected_utilization_rho: expected_util,
            service_rate_mu: tier.service_rate_mu,
            aggregate_arrival_rate_lambda: lambda_j,
            rho_queue,
            expected_wait_seconds: expected_wait,
            energy_used_kWh: energy_kwh,
            carbon_used_kgCO2e: carbon_kg,
            within_storage_envelope: within_storage,
            within_eco_envelope: within_eco,
            within_latency_envelope: within_latency,
        });
    }

    // Logical capacity bound: approximate single bottleneck tier.
    let mut n_logical_max = f64::INFINITY;
    for tr in &tier_reports {
        if tr.capacity_TiB <= 0.0 || tr.expected_storage_TiB <= 0.0 {
            continue;
        }
        let headroom_tib = (tr.capacity_TiB * tr.target_utilization_rho)
            .saturating_sub(tr.expected_storage_TiB);
        if headroom_tib <= 0.0 {
            n_logical_max = n_logical_max.min(0.0);
            continue;
        }
        // Assume average SKO effective size ~ total_expected_bytes / (lambda_total * T).
        let mean_eff_bytes_per_sko = if class_total_lambda > 0.0 && horizon_hours > 0.0 {
            total_expected_bytes / (class_total_lambda * horizon_hours)
        } else {
            0.0
        };
        if mean_eff_bytes_per_sko > 0.0 {
            let headroom_bytes = headroom_tib * 1024.0_f64.powi(4);
            let n_for_tier = headroom_bytes / mean_eff_bytes_per_sko;
            n_logical_max = n_logical_max.min(n_for_tier);
        }
    }

    if !n_logical_max.is_finite() {
        n_logical_max = 0.0;
    }

    let kappa_ratio = if total_physical_bytes > 0.0 {
        total_expected_bytes / total_physical_bytes
    } else {
        0.0
    };

    // Eco-wealth: utility per unit carbon.
    let total_carbon: f64 = tier_carbon_kg.values().copied().sum();
    let ew_den = if total_carbon > 0.0 { total_carbon } else { 1e-9 };
    let ew_score = class_weighted_utility / ew_den;

    // RoH: leave as a placeholder function for now.
    let roh_score = estimate_roh_from_loads(&tier_reports);
    let roh_ceiling = placement.roh_ceiling;
    let within_roh = roh_score <= roh_ceiling;

    let mut class_proportions: HashMap<String, f64> = HashMap::new();
    if class_total_lambda > 0.0 {
        for c in &sko_model.classes {
            let lambda_k = *class_lambda.get(&c.id).unwrap_or(&0.0);
            class_proportions.insert(c.id.clone(), lambda_k / class_total_lambda);
        }
    }

    let logical_summary = LogicalCapacitySummary {
        n_logical_max,
        kappa_ratio,
        class_proportions,
        ew_numerator: class_weighted_utility,
        ew_denominator: ew_den,
        ew_score,
        roh_score,
        roh_ceiling,
        within_roh_ceiling: within_roh,
    };

    CapacityReport {
        horizon_hours,
        tier_reports,
        logical_summary,
    }
}

fn estimate_energy_and_carbon(
    tier: &crate::capacity_specs::TierCapacity,
    utilization: f64,
    horizon_hours: f64,
) -> (f64, f64) {
    let u = utilization.clamp(0.0, 1.0);
    let p_idle = tier.eco.power_idle_W_per_TB;
    let p_active = tier.eco.power_active_W_per_TB;
    let pue = tier.eco.pue;
    let ci = tier.eco.grid_carbon_intensity_kg_per_kWh;

    // Approximate power per TB over the horizon.
    let p_effective = p_idle + u * (p_active - p_idle);
    let energy_kwh = p_effective * horizon_hours / 1000.0 * pue;
    let carbon_kg = energy_kwh * ci;

    (energy_kwh, carbon_kg)
}

/// Simple heuristic: derive latency SLA per tier from placement plan.
fn derive_latency_sla_for_tier(tier_id: &str, plan: &PlacementPlan) -> f64 {
    let mut min_sla = f64::INFINITY;
    for entry in &plan.entries {
        for frac in &entry.tiers {
            if frac.tier_id == tier_id && frac.fraction > 0.0 {
                if entry.latency_sla_seconds < min_sla {
                    min_sla = entry.latency_sla_seconds;
                }
            }
        }
    }
    if min_sla.is_finite() {
        min_sla
    } else {
        // Fallback: 2 * default latency.
        2.0
    }
}

/// Placeholder RoH estimator: you can later plug in your full RoH model.
fn estimate_roh_from_loads(tiers: &[TierReport]) -> f64 {
    // Example: treat any over-utilization or eco violation as additive risk.
    let mut risk = 0.0;
    for t in tiers {
        if !t.within_storage_envelope {
            risk += 0.1;
        }
        if !t.within_eco_envelope {
            risk += 0.1;
        }
        if !t.within_latency_envelope {
            risk += 0.1;
        }
    }
    risk.clamp(0.0, 1.0)
}
