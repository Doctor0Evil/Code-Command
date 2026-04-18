// FILE: ./src/catalog/embodied_carbon.rs
// CC-LANG: rs
// CC-FILE: src/catalog/embodied_carbon.rs
// CC-DEEP: ./src/catalog/embodied_carbon.rs

#[derive(Clone, Debug)]
pub struct EmbodiedProfile {
    pub generation_id: String,
    pub tier_id: String,
    pub embodied_kg_co2e_per_tb: f64,
    pub design_lifetime_years: f64,
}

impl EmbodiedProfile {
    pub fn annualized_kg_co2e_per_tb(&self) -> f64 {
        let denom = if self.design_lifetime_years <= 0.0 {
            1.0
        } else {
            self.design_lifetime_years
        };
        self.embodied_kg_co2e_per_tb / denom
    }

    pub fn per_request_kg_co2e(&self, logical_tb_years: f64) -> f64 {
        if logical_tb_years <= 0.0 {
            0.0
        } else {
            self.annualized_kg_co2e_per_tb() * logical_tb_years
        }
    }
}

pub struct EmbodiedCatalog {
    entries: Vec<EmbodiedProfile>,
}

impl EmbodiedCatalog {
    pub fn new(entries: Vec<EmbodiedProfile>) -> Self {
        Self { entries }
    }

    pub fn find_by_generation(&self, generation_id: &str) -> Option<EmbodiedProfile> {
        for e in &self.entries {
            if e.generation_id == generation_id {
                return Some(e.clone());
            }
        }
        None
    }
}
