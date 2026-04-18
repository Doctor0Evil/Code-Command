// FILE: ./src/catalog/grid_carbon_catalog.rs
// CC-LANG: rs
// CC-FILE: src/catalog/grid_carbon_catalog.rs
// CC-DEEP: ./src/catalog/grid_carbon_catalog.rs
// CC-SOV: no external crates

use std::collections::BTreeMap;

/// Trivial timestamp wrapper (seconds since Unix epoch) to avoid external time crates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleTimestamp {
    pub seconds_since_epoch: i64,
}

impl SimpleTimestamp {
    pub fn new(seconds_since_epoch: i64) -> Self {
        Self { seconds_since_epoch }
    }
}

#[derive(Clone, Debug)]
pub struct GridSample {
    pub ts: SimpleTimestamp,
    pub carbon_g_per_kwh: f64,
    pub renewable_fraction: f64, // 0.0 – 1.0
}

#[derive(Clone, Debug)]
pub struct RegionTimeSeries {
    pub region_id: String,
    pub samples: BTreeMap<SimpleTimestamp, GridSample>,
}

#[derive(Clone, Debug)]
pub enum GridCarbonError {
    RegionNotFound(String),
    NoSample { region_id: String, ts: SimpleTimestamp },
}

pub trait GridCarbonCatalog: Send + Sync {
    fn get_latest_before(
        &self,
        region_id: &str,
        ts: SimpleTimestamp,
    ) -> Result<GridSample, GridCarbonError>;

    fn get_window(
        &self,
        region_id: &str,
        start: SimpleTimestamp,
        end: SimpleTimestamp,
    ) -> Result<Vec<GridSample>, GridCarbonError>;
}

pub struct InMemoryGridCarbonCatalog {
    regions: BTreeMap<String, RegionTimeSeries>,
}

impl InMemoryGridCarbonCatalog {
    pub fn new(regions: Vec<RegionTimeSeries>) -> Self {
        let mut map = BTreeMap::new();
        for r in regions {
            map.insert(r.region_id.clone(), r);
        }
        Self { regions: map }
    }
}

impl GridCarbonCatalog for InMemoryGridCarbonCatalog {
    fn get_latest_before(
        &self,
        region_id: &str,
        ts: SimpleTimestamp,
    ) -> Result<GridSample, GridCarbonError> {
        let r = match self.regions.get(region_id) {
            Some(r) => r,
            None => {
                return Err(GridCarbonError::RegionNotFound(
                    region_id.to_string(),
                ))
            }
        };

        let mut candidate: Option<&GridSample> = None;
        for (k, v) in &r.samples {
            if *k <= ts {
                candidate = Some(v);
            } else {
                break;
            }
        }
        match candidate {
            Some(v) => Ok(v.clone()),
            None => Err(GridCarbonError::NoSample {
                region_id: region_id.to_string(),
                ts,
            }),
        }
    }

    fn get_window(
        &self,
        region_id: &str,
        start: SimpleTimestamp,
        end: SimpleTimestamp,
    ) -> Result<Vec<GridSample>, GridCarbonError> {
        let r = match self.regions.get(region_id) {
            Some(r) => r,
            None => {
                return Err(GridCarbonError::RegionNotFound(
                    region_id.to_string(),
                ))
            }
        };

        let mut out = Vec::new();
        for (k, v) in r.samples.range(start..=end) {
            let _ = k;
            out.push(v.clone());
        }

        if out.is_empty() {
            Err(GridCarbonError::NoSample {
                region_id: region_id.to_string(),
                ts: start,
            })
        } else {
            Ok(out)
        }
    }
}
