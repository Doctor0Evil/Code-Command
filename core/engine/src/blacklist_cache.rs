/// BlacklistCache: Caching layer for blacklist scan results
///
/// Implements DR84/DR55 semantics for caching blacklist scan results to avoid
/// rescanning unchanged files when rules are unchanged.
///
/// Key design points:
/// - Keys are normalized VFS paths (consistent with CC-PATH and VfsSnapshot path semantics)
/// - hash: current file content hash (SHA from VfsSnapshot/TaskQueue writefile task)
/// - rules_version: monotonically increasing ruleset version, incremented on policy reload
/// - timestamp: epoch seconds when the scan was performed
/// - matches: all BlacklistMatch instances (including empty case)
/// - ttl: time-to-live in seconds after which entry expires and must be ignored
///
/// Invariants (DR55 algorithm):
/// - Entry is valid only when: hash == current_hash AND rules_version == current_rules_version AND now < timestamp + ttl
/// - On policy reload (specs/blacklist.aln or .ccblacklist.aln), rules_version increments
/// - Expired entries (now >= timestamp + ttl) are treated as stale and ignored

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single match result from blacklist scanning
#[derive(Debug, Clone)]
pub struct BlacklistMatch {
    /// The rule ID that matched
    pub rule_id: String,
    /// The pattern/token that matched
    pub pattern: String,
    /// Start byte offset in the scanned content
    pub start: usize,
    /// End byte offset in the scanned content
    pub end: usize,
    /// Severity level ("block", "warn", "report")
    pub severity: String,
    /// Reason/explanation for the rule
    pub reason: String,
}

/// Cache entry for a single file's blacklist scan results
///
/// Fields per DR84/DR55:
/// - hash: current file content hash at scan time
/// - rules_version: ruleset version at scan time
/// - timestamp: epoch seconds when scan was performed
/// - matches: all BlacklistMatch instances from the scan
/// - ttl: time-to-live in seconds
#[derive(Debug, Clone)]
pub struct BlacklistCacheEntry {
    /// Content hash of the file at scan time
    pub hash: String,
    /// Rules version at scan time (monotonically increasing)
    pub rules_version: u64,
    /// Epoch seconds when the scan was performed
    pub timestamp: u64,
    /// All matches found (empty Vec if no matches)
    pub matches: Vec<BlacklistMatch>,
    /// Time-to-live in seconds
    pub ttl: u64,
}

impl BlacklistCacheEntry {
    /// Create a new cache entry
    pub fn new(
        hash: String,
        rules_version: u64,
        matches: Vec<BlacklistMatch>,
        ttl: u64,
    ) -> Self {
        let timestamp = Self::current_timestamp();
        BlacklistCacheEntry {
            hash,
            rules_version,
            timestamp,
            matches,
            ttl,
        }
    }

    /// Check if this entry is still valid per DR55 algorithm
    ///
    /// Returns true only when:
    /// - hash == current_hash (file content unchanged)
    /// - rules_version == current_rules_version (rules unchanged)
    /// - now < timestamp + ttl (entry not expired)
    pub fn is_valid(&self, current_hash: &str, current_rules_version: u64, now: u64) -> bool {
        self.hash == current_hash
            && self.rules_version == current_rules_version
            && now < self.timestamp + self.ttl
    }

    /// Get current Unix timestamp in seconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Cache for blacklist scan results keyed by normalized VFS paths
///
/// Keys are normalized paths consistent with CC-PATH and VfsSnapshot path semantics.
/// The cache avoids rescanning unchanged files when blacklist rules are unchanged.
#[derive(Debug)]
pub struct BlacklistCache {
    /// Map from normalized path to cache entry
    entries: HashMap<String, BlacklistCacheEntry>,
    /// Default TTL for new entries (in seconds)
    default_ttl: u64,
}

impl BlacklistCache {
    /// Create a new blacklist cache with default TTL
    pub fn new(default_ttl_seconds: u64) -> Self {
        BlacklistCache {
            entries: HashMap::new(),
            default_ttl: default_ttl_seconds,
        }
    }

    /// Create a new cache with default settings (5 minute TTL)
    pub fn default_settings() -> Self {
        Self::new(300) // 5 minutes
    }

    /// Insert a cache entry for a path
    ///
    /// The path will be normalized before use as a key.
    pub fn insert(&mut self, path: &str, entry: BlacklistCacheEntry) {
        let normalized = normalize_path(path);
        self.entries.insert(normalized, entry);
    }

    /// Store scan results in the cache
    ///
    /// Creates an entry with the given hash, rules_version, matches, and default TTL.
    pub fn store(
        &mut self,
        path: &str,
        hash: &str,
        rules_version: u64,
        matches: Vec<BlacklistMatch>,
    ) {
        let normalized = normalize_path(path);
        let entry = BlacklistCacheEntry::new(
            hash.to_string(),
            rules_version,
            matches,
            self.default_ttl,
        );
        self.entries.insert(normalized, entry);
    }

    /// Look up a valid cache entry per DR55 algorithm
    ///
    /// Returns Some(entry) only when:
    /// - An entry exists for the path
    /// - entry.hash == current_hash
    /// - entry.rules_version == current_rules_version
    /// - now < entry.timestamp + entry.ttl
    ///
    /// Returns None if entry is missing or invalid (stale/expired).
    pub fn lookup_valid(
        &self,
        path: &str,
        current_hash: &str,
        current_rules_version: u64,
        now: u64,
    ) -> Option<&BlacklistCacheEntry> {
        let normalized = normalize_path(path);
        self.entries.get(&normalized).and_then(|entry| {
            if entry.is_valid(current_hash, current_rules_version, now) {
                Some(entry)
            } else {
                None
            }
        })
    }

    /// Remove an entry from the cache
    pub fn remove(&mut self, path: &str) {
        let normalized = normalize_path(path);
        self.entries.remove(&normalized);
    }

    /// Clear all entries from the cache
    ///
    /// Called when rules_version changes to force rescan of all files.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Invalidate entries by removing those with old rules_version
    ///
    /// This is a more targeted invalidation than clear(), keeping entries
    /// that might still be valid under certain migration scenarios.
    pub fn invalidate_old_rules(&mut self, current_rules_version: u64) {
        self.entries
            .retain(|_, entry| entry.rules_version >= current_rules_version);
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove expired entries (garbage collection)
    ///
    /// Call periodically to prevent unbounded growth.
    pub fn gc_expired(&mut self, now: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| now < entry.timestamp + entry.ttl);
        before - self.entries.len()
    }
}

impl Default for BlacklistCache {
    fn default() -> Self {
        Self::default_settings()
    }
}

/// Normalize a path for use as cache key
///
/// Consistent with CC-PATH and VfsSnapshot path semantics:
/// - Removes redundant slashes
/// - Resolves . and .. components
/// - Produces a canonical string representation
fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        let trimmed = segment.trim();
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }
        if trimmed == ".." {
            parts.pop();
            continue;
        }
        parts.push(trimmed);
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blacklist_cache_entry_construction() {
        let matches = vec![
            BlacklistMatch {
                rule_id: "rule-1".to_string(),
                pattern: "forbidden_token".to_string(),
                start: 10,
                end: 25,
                severity: "block".to_string(),
                reason: "Security risk".to_string(),
            },
        ];

        let entry = BlacklistCacheEntry::new(
            "abc123".to_string(),
            5,
            matches.clone(),
            300,
        );

        assert_eq!(entry.hash, "abc123");
        assert_eq!(entry.rules_version, 5);
        assert_eq!(entry.ttl, 300);
        assert_eq!(entry.matches.len(), 1);
        assert_eq!(entry.matches[0].rule_id, "rule-1");
    }

    #[test]
    fn test_is_valid_matching_conditions() {
        let entry = BlacklistCacheEntry::new(
            "hash123".to_string(),
            10,
            vec![],
            300,
        );

        let now = entry.timestamp + 10; // 10 seconds after creation

        // All conditions match - should be valid
        assert!(entry.is_valid("hash123", 10, now));

        // Hash mismatch - invalid
        assert!(!entry.is_valid("different_hash", 10, now));

        // Rules version mismatch - invalid
        assert!(!entry.is_valid("hash123", 11, now));
    }

    #[test]
    fn test_is_valid_expired_ttl() {
        let entry = BlacklistCacheEntry::new(
            "hash123".to_string(),
            10,
            vec![],
            60, // 60 second TTL
        );

        let now = entry.timestamp + 100; // 100 seconds after creation (expired)

        // TTL expired - invalid even though hash and rules match
        assert!(!entry.is_valid("hash123", 10, now));

        // Just before expiry - valid
        let just_before = entry.timestamp + 59;
        assert!(entry.is_valid("hash123", 10, just_before));
    }

    #[test]
    fn test_blacklist_cache_insert_and_lookup() {
        let mut cache = BlacklistCache::new(300);

        let matches = vec![BlacklistMatch {
            rule_id: "rule-1".to_string(),
            pattern: "test".to_string(),
            start: 0,
            end: 4,
            severity: "warn".to_string(),
            reason: "Test".to_string(),
        }];

        cache.store("/path/to/file.rs", "hash1", 5, matches.clone());

        let now = cache.entries.get(&normalize_path("/path/to/file.rs")).unwrap().timestamp + 10;

        // Valid lookup
        let entry = cache.lookup_valid("/path/to/file.rs", "hash1", 5, now);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().matches.len(), 1);

        // Invalid due to hash change
        let entry = cache.lookup_valid("/path/to/file.rs", "hash2", 5, now);
        assert!(entry.is_none());

        // Invalid due to rules version change
        let entry = cache.lookup_valid("/path/to/file.rs", "hash1", 6, now);
        assert!(entry.is_none());
    }

    #[test]
    fn test_path_normalization() {
        assert_eq!(normalize_path("/foo/bar"), "foo/bar");
        assert_eq!(normalize_path("foo//bar"), "foo/bar");
        assert_eq!(normalize_path("foo/./bar"), "foo/bar");
        assert_eq!(normalize_path("foo/baz/../bar"), "foo/bar");
        assert_eq!(normalize_path("./foo/bar"), "foo/bar");
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = BlacklistCache::new(300);
        cache.store("/file1.rs", "h1", 1, vec![]);
        cache.store("/file2.rs", "h2", 1, vec![]);

        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_gc_expired() {
        let mut cache = BlacklistCache::new(60); // 60 second TTL
        cache.store("/file1.rs", "h1", 1, vec![]);
        
        // Manually adjust timestamp to simulate old entry
        let normalized = normalize_path("/file1.rs");
        if let Some(entry) = cache.entries.get_mut(&normalized) {
            entry.timestamp = 0; // Very old
        }

        let now = 1000; // Far in the future
        let removed = cache.gc_expired(now);
        assert_eq!(removed, 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_round_trip_fields() {
        let original_matches = vec![
            BlacklistMatch {
                rule_id: "r1".to_string(),
                pattern: "p1".to_string(),
                start: 5,
                end: 10,
                severity: "block".to_string(),
                reason: "reason1".to_string(),
            },
            BlacklistMatch {
                rule_id: "r2".to_string(),
                pattern: "p2".to_string(),
                start: 20,
                end: 25,
                severity: "warn".to_string(),
                reason: "reason2".to_string(),
            },
        ];

        let mut cache = BlacklistCache::new(300);
        cache.store(
            "/test/path.rs",
            "sha256:abc123",
            42,
            original_matches.clone(),
        );

        let normalized = normalize_path("/test/path.rs");
        let entry = cache.entries.get(&normalized).unwrap();

        // Verify all fields round-trip correctly
        assert_eq!(entry.hash, "sha256:abc123");
        assert_eq!(entry.rules_version, 42);
        assert_eq!(entry.matches.len(), 2);
        
        for (orig, stored) in original_matches.iter().zip(entry.matches.iter()) {
            assert_eq!(orig.rule_id, stored.rule_id);
            assert_eq!(orig.pattern, stored.pattern);
            assert_eq!(orig.start, stored.start);
            assert_eq!(orig.end, stored.end);
            assert_eq!(orig.severity, stored.severity);
            assert_eq!(orig.reason, stored.reason);
        }
    }
}
