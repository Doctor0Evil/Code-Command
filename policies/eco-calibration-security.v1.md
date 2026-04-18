# FILE: ./policies/eco-calibration-security.v1.md
# CC-LANG: md
# CC-FILE: policies/eco-calibration-security.v1.md
# CC-DEEP: ./policies/eco-calibration-security.v1.md

# Eco Calibration Security Policy (v1)

1. Scope  
   This policy governs ingestion and use of grid carbon, P_idle/P_active/PUE,
   embodied carbon, vault risk metrics, and SKO workload distributions inside
   the eco-placement and calibration modules.

2. Integrity and Provenance  
   - All catalog files (`grid-carbon-catalog.v1.yaml`, `hardware-catalog.v1.yaml`,
     `embodied-carbon.v1.yaml`, `vault-risk-posture.v1.yaml`,
     `sko-workload-classes.v1.yaml`, `workload-distribution.v1.yaml`) must be
     committed via a signed, reviewed change in the Code-Command repository.  
   - The engine loads only catalog snapshots from trusted branches; runtime
     mutation of catalogs is forbidden.

3. Privilege Boundaries  
   - Calibration binaries such as `calibrate_eco_placement` run in a
     non-privileged context and cannot modify live placement decisions.  
   - Live placement or eco-scoring services consume only validated snapshots
     written by CI after policy checks.

4. RoH Invariants  
   - No calibration or optimization code may alter RoH ceilings, tag algebra,
     or feasibility rules.  
   - The risk model exposes a single monotone interface (`is_allowed`) that
     upstream code must respect; eco-cost functions operate strictly on
     already-feasible candidates.

5. Data Minimization  
   - Workload distributions and SKO class parameters are aggregated and
     de-identified before use in calibration.  
   - Individual SKO identifiers or user-level records are excluded from
     calibration datasets.

6. Auditability  
   - Each placement decision logs the catalog versions and RoH decision path
     used.  
   - Calibration runs are recorded with input catalog hashes and outputs, so
     eco-wealth changes can be audited against configuration history.
