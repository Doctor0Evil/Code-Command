// FILE: ./src/catalog/hardware_power.rs
// CC-LANG: rs
// CC-FILE: src/catalog/hardware_power.rs
// CC-DEEP: ./src/catalog/hardware_power.rs

#[derive(Clone, Debug)]
pub struct TierPowerProfile {
    pub tier_id: String,
    pub p_idle_w_per_tb: f64,
    pub p_active_w_per_tb: f64,
    pub pue: f64,
}

impl TierPowerProfile {
    /// Linear model: P(u) ≈ P_idle + u * (P_active - P_idle), scaled by capacity and PUE.
    pub fn power_w(&self, utilization: f64, capacity_tb: f64) -> f64 {
        let mut u = utilization;
        if u < 0.0 {
            u = 0.0;
        }
        if u > 1.0 {
            u = 1.0;
        }

        let p_device = self.p_idle_w_per_tb
            + u * (self.p_active_w_per_tb - self.p_idle_w_per_tb);
        p_device * capacity_tb * self.pue
    }
}

pub trait HardwarePowerCatalog: Send + Sync {
    fn get_tier(&self, tier_id: &str) -> Option<TierPowerProfile>;
}

pub struct InMemoryHardwarePowerCatalog {
    tiers: Vec<TierPowerProfile>,
}

impl InMemoryHardwarePowerCatalog {
    pub fn new(tiers: Vec<TierPowerProfile>) -> Self {
        Self { tiers }
    }
}

impl HardwarePowerCatalog for InMemoryHardwarePowerCatalog {
    fn get_tier(&self, tier_id: &str) -> Option<TierPowerProfile> {
        for t in &self.tiers {
            if t.tier_id == tier_id {
                return Some(t.clone());
            }
        }
        None
    }
}
