// FILE: ./src/workload/sko_workload.rs
// CC-LANG: rs
// CC-FILE: src/workload/sko_workload.rs
// CC-DEEP: ./src/workload/sko_workload.rs

#[derive(Clone, Debug)]
pub struct SkoClassDescriptor {
    pub class_id: String,
    pub sensitivity: f64,
    pub typical_size_mb: f64,
    pub typical_read_ratio: f64,
    pub typical_write_ratio: f64,
    pub target_latency_ms_p95: u64,
    pub deferral_window_minutes: u64,
}

#[derive(Clone, Debug)]
pub struct RegionWorkloadDistribution {
    pub region_id: String,
    pub total_skos: u64,
    pub fractions: Vec<(String, f64)>,
}

pub struct WorkloadRegistry {
    pub classes: Vec<SkoClassDescriptor>,
    pub region_distributions: Vec<RegionWorkloadDistribution>,
}

impl WorkloadRegistry {
    pub fn expected_arrival_rate(
        &self,
        region_id: &str,
        class_id: &str,
    ) -> Option<f64> {
        let region = self
            .region_distributions
            .iter()
            .find(|r| r.region_id == region_id)?;
        let frac_entry = region
            .fractions
            .iter()
            .find(|(cid, _)| cid == class_id)?;
        let fraction = frac_entry.1;
        let total = region.total_skos as f64;
        let base_lambda = total * fraction / 10_000.0;
        Some(base_lambda / 60.0)
    }

    pub fn find_class(&self, class_id: &str) -> Option<SkoClassDescriptor> {
        for c in &self.classes {
            if c.class_id == class_id {
                return Some(c.clone());
            }
        }
        None
    }
}
