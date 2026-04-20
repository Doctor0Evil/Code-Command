// FILE .core/engine/src/tests_engine_contract.rs
//
// Tests for alignment between .specs/engine.aln and the Rust implementation.
// These tests assume the engine is compiled as a native crate for testing,
// not via WASM, and call functions directly.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cc_engine_identity, cc_engine_capabilities, cc_init_vfs, cc_last_error_code};

    #[test]
    fn engine_identity_matches_spec() {
        let id_json = cc_engine_identity();
        assert!(
            id_json.contains("\"id\":\"cc-engine1\""),
            "Engine id must be cc-engine1"
        );
        assert!(
            id_json.contains("\"version\":\"1.0.0\""),
            "Engine version must be 1.0.0"
        );
        assert!(
            id_json.contains("\"status\":\"active\""),
            "Engine status must be active"
        );
        assert!(
            id_json.contains("\"languages\":[\"rust\",\"cpp\"]"),
            "Engine languages must include rust and cpp"
        );
        assert!(
            id_json.contains("\"CC-VOL\"") && id_json.contains("\"CC-PATH\""),
            "Engine invariants must include all 10 CC- tags"
        );
    }

    #[test]
    fn engine_capabilities_include_identity_and_features() {
        let caps = cc_engine_capabilities();
        assert!(
            caps.contains("\"engine\":"),
            "Capabilities must embed engine identity"
        );
        assert!(
            caps.contains("\"features\":[\"token-bench\",\"connector-sandbox\"]"),
            "Capabilities must list the expected feature flags"
        );
        assert!(
            caps.contains("\"profiles\":[\"github\",\"local\",\"memory-only\"]"),
            "Capabilities must list supported profiles"
        );
        assert!(
            caps.contains("\"tags\":[") && caps.contains("\"CC-SOV\""),
            "Capabilities must list all CC- tags"
        );
    }

    #[test]
    fn init_vfs_rejects_wrong_version() {
        // Snapshot with wrong version field.
        let snapshot = r#"{"version":"VFS-SNAPSHOT-0","files":[]}"#;
        let ok = cc_init_vfs(snapshot);
        assert!(!ok, "Init VFS must fail on version mismatch");
        let code = cc_last_error_code();
        assert_eq!(code, "CCENG-010");
    }

    #[test]
    fn init_vfs_accepts_correct_version() {
        let snapshot = r#"{"version":"VFS-SNAPSHOT-1","files":[]}"#;
        let ok = cc_init_vfs(snapshot);
        assert!(ok, "Init VFS must accept VFS-SNAPSHOT-1");
        let code = cc_last_error_code();
        assert_eq!(code, "CCENG-000");
    }

    #[test]
    fn unknown_profile_sets_error_code() {
        // Minimal payload with unknown profile.
        let payload = r#"{
          "version": "TASK-LANGUAGE-1.0",
          "profile": "unknown-profile",
          "tasks": []
        }"#;
        let report = crate::cc_execute_task(payload);
        assert!(
            report.contains("\"ok\":false"),
            "TaskReport must indicate failure for unknown profile"
        );
        let code = cc_last_error_code();
        assert_eq!(code, "CCENG-030");
    }

    #[test]
    fn unknown_task_kind_causes_cceng_002_and_no_panic() {
        // Payload with a single unknown task kind.
        let payload = r#"{
          "version": "TASK-LANGUAGE-1.0",
          "tasks": [
            {
              "kind": "unknown-kind",
              "path": "src/main.rs",
              "content": "",
              "sha": "",
              "tags": []
            }
          ]
        }"#;
        let report = crate::cc_execute_task(payload);
        assert!(
            report.contains("\"ok\":false"),
            "TaskReport must indicate failure for unknown task kind"
        );
        let code = cc_last_error_code();
        assert_eq!(code, "CCENG-002");
    }

    #[test]
    fn cc_last_error_resets_on_success() {
        // Force an error first.
        let snapshot = r#"{"version":"VFS-SNAPSHOT-0","files":[]}"#;
        let _ = cc_init_vfs(snapshot);
        assert_eq!(cc_last_error_code(), "CCENG-010");

        // Then a success.
        let snapshot_ok = r#"{"version":"VFS-SNAPSHOT-1","files":[]}"#;
        let _ = cc_init_vfs(snapshot_ok);
        assert_eq!(cc_last_error_code(), "CCENG-000");
    }
}
