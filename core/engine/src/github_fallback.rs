// FILE: ./core/engine/src/github_fallback.rs

//! GitHub API fallback layer for Code-Command.
//! Provides status reporting and health checks for the GitHub API integration.
//! This module exposes WASM functions that the UI can call to determine API health.

use wasm_bindgen::prelude::*; // Build-time JS glue only per design. [file:2]

/// Represents the current GitHub API health status. [file:2]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GithubApiStatus {
    /// API is fully operational
    Online,
    /// API is experiencing issues but functional
    Degraded,
    /// API is rate-limited or temporarily unavailable
    Limited,
    /// API is completely unreachable
    Offline,
}

impl GithubApiStatus {
    fn as_str(&self) -> &'static str {
        match self {
            GithubApiStatus::Online => "online",
            GithubApiStatus::Degraded => "degraded",
            GithubApiStatus::Limited => "limited",
            GithubApiStatus::Offline => "offline",
        }
    }
}

/// Static status holder (updated by JS via bridge). [file:2]
static mut CURRENT_STATUS: GithubApiStatus = GithubApiStatus::Online;
static mut LAST_CHECK_TIMESTAMP: u64 = 0;
static mut FAILURE_COUNT: u32 = 0;
static mut CACHE_HIT_COUNT: u32 = 0;

/// Update the GitHub API status from JavaScript. [file:2]
#[wasm_bindgen]
pub fn cc_update_github_status(status: &str) {
    let new_status = match status {
        "online" => GithubApiStatus::Online,
        "degraded" => GithubApiStatus::Degraded,
        "limited" => GithubApiStatus::Limited,
        "offline" => GithubApiStatus::Offline,
        _ => GithubApiStatus::Degraded,
    };

    unsafe {
        CURRENT_STATUS = new_status;
        LAST_CHECK_TIMESTAMP = js_timestamp_ms();
    }
}

/// Record an API failure (called from JS when fetch fails). [file:2]
#[wasm_bindgen]
pub fn cc_record_api_failure() {
    unsafe {
        FAILURE_COUNT += 1;
        if FAILURE_COUNT >= 3 {
            CURRENT_STATUS = GithubApiStatus::Limited;
        }
    }
}

/// Record an API success (called from JS when fetch succeeds). [file:2]
#[wasm_bindgen]
pub fn cc_record_api_success() {
    unsafe {
        FAILURE_COUNT = 0;
        if CURRENT_STATUS == GithubApiStatus::Limited {
            CURRENT_STATUS = GithubApiStatus::Degraded;
        }
    }
}

/// Record a cache hit (for telemetry). [file:2]
#[wasm_bindgen]
pub fn cc_record_cache_hit() {
    unsafe {
        CACHE_HIT_COUNT += 1;
    }
}

/// Get the current GitHub API status as a JSON string. [file:2]
/// Returns: {"status": "online"|"degraded"|"limited"|"offline", "failureCount": N, "cacheHits": N, "lastCheck": timestamp}
#[wasm_bindgen]
pub fn cc_github_status() -> String {
    unsafe {
        format!(
            r#"{{"status":"{}","failureCount":{},"cacheHits":{},"lastCheck":{}}}"#,
            CURRENT_STATUS.as_str(),
            FAILURE_COUNT,
            CACHE_HIT_COUNT,
            LAST_CHECK_TIMESTAMP
        )
    }
}

/// Check if the API is currently available for writes. [file:2]
#[wasm_bindgen]
pub fn cc_is_github_api_available() -> bool {
    unsafe {
        CURRENT_STATUS == GithubApiStatus::Online || CURRENT_STATUS == GithubApiStatus::Degraded
    }
}

/// Get a human-readable status message. [file:2]
#[wasm_bindgen]
pub fn cc_get_github_status_message() -> String {
    unsafe {
        match CURRENT_STATUS {
            GithubApiStatus::Online => "GitHub API operational".to_string(),
            GithubApiStatus::Degraded => "GitHub API experiencing minor issues".to_string(),
            GithubApiStatus::Limited => "GitHub API rate-limited - using cached data".to_string(),
            GithubApiStatus::Offline => "GitHub API unreachable".to_string(),
        }
    }
}

/// Reset all status counters (for testing or manual reset). [file:2]
#[wasm_bindgen]
pub fn cc_reset_github_status() {
    unsafe {
        CURRENT_STATUS = GithubApiStatus::Online;
        FAILURE_COUNT = 0;
        CACHE_HIT_COUNT = 0;
        LAST_CHECK_TIMESTAMP = 0;
    }
}

/* ---------- Helper Functions ---------- */ [file:2]

/// Get current timestamp in milliseconds (placeholder - JS should provide this). [file:2]
fn js_timestamp_ms() -> u64 {
    // In practice, this would be called via JS shim with Date.now()
    // For now, return 0 and let JS update timestamps via cc_update_github_status
    0
}

/* ---------- Unit Tests ---------- */ [file:2]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_as_str() {
        assert_eq!(GithubApiStatus::Online.as_str(), "online");
        assert_eq!(GithubApiStatus::Degraded.as_str(), "degraded");
        assert_eq!(GithubApiStatus::Limited.as_str(), "limited");
        assert_eq!(GithubApiStatus::Offline.as_str(), "offline");
    }

    #[test]
    fn test_status_transitions() {
        cc_reset_github_status();
        
        // Initially online
        let status_json = cc_github_status();
        assert!(status_json.contains("\"status\":\"online\""));

        // Record failures
        cc_record_api_failure();
        cc_record_api_failure();
        cc_record_api_failure();

        // Should be limited after 3 failures
        let status_json = cc_github_status();
        assert!(status_json.contains("\"status\":\"limited\""));

        // Reset
        cc_reset_github_status();
        let status_json = cc_github_status();
        assert!(status_json.contains("\"status\":\"online\""));
    }

    #[test]
    fn test_availability_check() {
        cc_reset_github_status();
        assert!(cc_is_github_api_available());

        // Simulate limited status
        cc_record_api_failure();
        cc_record_api_failure();
        cc_record_api_failure();
        assert!(!cc_is_github_api_available());
    }

    #[test]
    fn test_status_message() {
        cc_reset_github_status();
        assert_eq!(cc_get_github_status_message(), "GitHub API operational");

        cc_record_api_failure();
        cc_record_api_failure();
        cc_record_api_failure();
        assert!(cc_get_github_status_message().contains("rate-limited"));
    }
}
