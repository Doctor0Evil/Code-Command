// FILE: ./core/engine/src/cache_key.rs

/// Compute a deterministic cache key for `.cc-cache`
/// from `{owner, repo, ref_name, profile}` using FNV-1a 64-bit.
/// Format: `{owner}-{repo}-{profile}-{hash:x}`.
pub fn cache_key(owner: &str, repo: &str, ref_name: &str, profile: &str) -> String {
    let mut hasher = Fnv1a64::new();
    hasher.update(owner.as_bytes());
    hasher.update(b"/");
    hasher.update(repo.as_bytes());
    hasher.update(b"/");
    hasher.update(ref_name.as_bytes());
    hasher.update(b"/");
    hasher.update(profile.as_bytes());

    let hash = hasher.finish();
    format!("{owner}-{repo}-{profile}-{hash:x}")
}

struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    fn new() -> Self {
        // FNV-1a 64-bit offset basis
        Fnv1a64 {
            state: 0xcbf29ce484222325,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        // FNV-1a 64-bit prime
        const PRIME: u64 = 0x00000100000001b3;
        for &b in bytes {
            self.state ^= b as u64;
            self.state = self.state.wrapping_mul(PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

/// Compute a cache key for use by JS (e.g., IndexedDB namespaces)
/// Exposed in WASM as `cc_compute_cache_key`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn cc_compute_cache_key(owner: &str, repo: &str, ref_name: &str, profile: &str) -> String {
    cache_key(owner, repo, ref_name, profile)
}
