// FILE: ./js/app/github/api.js

// GitHub REST API wrapper for Code-Command. Uses only the official
// GitHub REST endpoints as a sovereign backend (no other external services). [file:2]

import * as Cache from "./cache.js";

const API_ROOT = "https://api.github.com";

/* ---------- Circuit Breaker & Retry Logic (Task 1 & Task 8) ---------- */ [file:2]

/**
 * CircuitBreaker class implements the circuit breaker pattern for GitHub API calls.
 * After 3 consecutive failures, the circuit opens and returns cached data or graceful errors.
 * The circuit half-opens after 30 seconds to test recovery.
 */
export class CircuitBreaker {
  constructor(options = {}) {
    this.failureThreshold = options.failureThreshold || 3;
    this.resetTimeout = options.resetTimeout || 30000; // 30 seconds
    this.state = 'closed'; // closed, open, half-open
    this.failures = 0;
    this.lastFailureTime = null;
    this.endpointFailures = new Map(); // Track failures per endpoint
  }

  /**
   * Record a failure for an endpoint.
   * @param {string} endpoint - The API endpoint (e.g., 'contents', 'trees')
   */
  recordFailure(endpoint) {
    const count = (this.endpointFailures.get(endpoint) || 0) + 1;
    this.endpointFailures.set(endpoint, count);
    this.failures = count;
    this.lastFailureTime = Date.now();

    if (count >= this.failureThreshold) {
      this.state = 'open';
      console.warn(`[CircuitBreaker] Circuit OPEN for ${endpoint} after ${count} failures`);
    }
  }

  /**
   * Record a success for an endpoint, resetting the counter.
   * @param {string} endpoint - The API endpoint
   */
  recordSuccess(endpoint) {
    this.endpointFailures.set(endpoint, 0);
    this.failures = 0;
    this.state = 'closed';
  }

  /**
   * Check if the circuit allows a request.
   * @param {string} endpoint - The API endpoint
   * @returns {{ allowed: boolean, state: string }}
   */
  canRequest(endpoint) {
    const now = Date.now();
    
    if (this.state === 'closed') {
      return { allowed: true, state: 'closed' };
    }

    if (this.state === 'open') {
      // Check if we should half-open
      if (now - this.lastFailureTime >= this.resetTimeout) {
        this.state = 'half-open';
        console.log(`[CircuitBreaker] Circuit HALF-OPEN for ${endpoint}, testing...`);
        return { allowed: true, state: 'half-open' };
      }
      return { allowed: false, state: 'open' };
    }

    if (this.state === 'half-open') {
      return { allowed: true, state: 'half-open' };
    }

    return { allowed: true, state: 'closed' };
  }

  /**
   * Get the current status of the circuit breaker.
   * @returns {{ state: string, failures: number, lastFailureTime: number|null }}
   */
  getStatus() {
    return {
      state: this.state,
      failures: this.failures,
      lastFailureTime: this.lastFailureTime,
    };
  }
}

// Global circuit breaker instance for all GitHub API calls
const circuitBreaker = new CircuitBreaker({ failureThreshold: 3, resetTimeout: 30000 });

/**
 * Fetch with retry logic and exponential backoff.
 * Retries up to 3 times with delays of 1s, 2s, 4s.
 * Falls back to cache on rate limit (403).
 * 
 * @param {string} url - The URL to fetch
 * @param {RequestInit} options - Fetch options
 * @param {string} cacheKey - Key for cache lookup on fallback
 * @returns {Promise<Response>}
 */
async function fetchWithRetry(url, options = {}, cacheKey = null) {
  const maxRetries = 3;
  let lastError = null;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      // Check circuit breaker before making request
      const endpoint = extractEndpoint(url);
      const circuitStatus = circuitBreaker.canRequest(endpoint);

      if (!circuitStatus.allowed) {
        // Circuit is open, try cache fallback
        if (cacheKey) {
          const cached = await Cache.get(cacheKey);
          if (cached) {
            console.log(`[fetchWithRetry] Circuit open, using cached data for ${endpoint}`);
            return createResponseFromCache(cached);
          }
        }
        throw new Error(`Circuit breaker open for ${endpoint}. Service temporarily unavailable.`);
      }

      const response = await fetch(url, options);

      // Handle rate limiting (403)
      if (response.status === 403) {
        const retryAfter = response.headers.get('Retry-After');
        const waitTime = retryAfter ? parseInt(retryAfter, 10) * 1000 : 60000;
        
        // Try cache fallback immediately on rate limit
        if (cacheKey) {
          const cached = await Cache.get(cacheKey);
          if (cached) {
            console.log(`[fetchWithRetry] Rate limited, using cached data`);
            return createResponseFromCache(cached);
          }
        }

        if (attempt < maxRetries) {
          const delay = Math.min(1000 * Math.pow(2, attempt), waitTime);
          console.warn(`[fetchWithRetry] Rate limited (403), retrying in ${delay}ms (attempt ${attempt + 1}/${maxRetries})`);
          await sleep(delay);
          continue;
        }

        circuitBreaker.recordFailure(endpoint);
        throw new Error(`GitHub API rate limit exceeded. Try again later.`);
      }

      // Handle server errors (5xx)
      if (response.status >= 500 && response.status < 600) {
        if (attempt < maxRetries) {
          const delay = 1000 * Math.pow(2, attempt); // Exponential backoff: 1s, 2s, 4s
          console.warn(`[fetchWithRetry] Server error (${response.status}), retrying in ${delay}ms (attempt ${attempt + 1}/${maxRetries})`);
          await sleep(delay);
          continue;
        }

        circuitBreaker.recordFailure(endpoint);
        throw new Error(`GitHub API server error (${response.status}). Please try again later.`);
      }

      // Success or client error (4xx other than 403)
      if (!response.ok && response.status !== 404) {
        circuitBreaker.recordFailure(endpoint);
      } else if (response.ok) {
        circuitBreaker.recordSuccess(endpoint);
      }

      return response;
    } catch (error) {
      lastError = error;
      
      // Don't retry on network errors if circuit is open
      if (error.message.includes('Circuit breaker')) {
        throw error;
      }

      if (attempt < maxRetries) {
        const delay = 1000 * Math.pow(2, attempt);
        console.warn(`[fetchWithRetry] Network error, retrying in ${delay}ms: ${error.message}`);
        await sleep(delay);
      }
    }
  }

  throw lastError || new Error('Fetch failed after all retries');
}

/**
 * Extract the endpoint type from a GitHub API URL.
 * @param {string} url - The API URL
 * @returns {string} - Endpoint identifier (e.g., 'contents', 'trees', 'commits')
 */
function extractEndpoint(url) {
  const match = url.match(/\/repos\/[^/]+\/[^/]+\/(\w+)/);
  return match ? match[1] : 'unknown';
}

/**
 * Create a Response object from cached data.
 * @param {*} data - Cached data
 * @returns {Response}
 */
function createResponseFromCache(data) {
  return new Response(JSON.stringify(data), {
    headers: { 'Content-Type': 'application/json' },
    status: 200,
  });
}

/**
 * Sleep for a specified duration.
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Get the current GitHub API health status.
 * @returns {{ status: string, circuitState: string, message: string }}
 */
export function getGithubStatus() {
  const status = circuitBreaker.getStatus();
  
  if (status.state === 'open') {
    return {
      status: 'limited',
      circuitState: status.state,
      message: 'GitHub API temporarily unavailable - using cached data where available',
    };
  } else if (status.failures > 0) {
    return {
      status: 'degraded',
      circuitState: status.state,
      message: `GitHub API experiencing issues (${status.failures} recent failures)`,
    };
  } else {
    return {
      status: 'online',
      circuitState: status.state,
      message: 'GitHub API operational',
    };
  }
}

// Expose status function globally for WASM bridge
window.cc_github_status = getGithubStatus;

/**
 * Fetches the full repository tree using the Git Trees API with ?recursive=1. [file:2]
 *
 * @param {string} owner
 * @param {string} repo
 * @param {string} sha optional tree SHA (defaults to "HEAD")
 * @returns {Promise<Array<{ path: string, type: "blob" | "tree", sha: string }>>}
 */
export async function fetchRepoTree(owner, repo, sha = "HEAD") {
  const cacheKey = `tree:${owner}/${repo}@${sha}`;
  const cached = await Cache.get(cacheKey);
  if (cached) {
    return cached;
  }

  // Resolve HEAD to a commit SHA first. [file:2]
  const refUrl = `${API_ROOT}/repos/${owner}/${repo}/git/refs/heads/${encodeURIComponent(
    "main"
  )}`;
  const refResp = await fetchWithRetry(refUrl, {}, cacheKey);
  let treeSha = sha;
  if (refResp.ok) {
    const refJson = await refResp.json();
    if (refJson && refJson.object && refJson.object.sha) {
      const commitSha = refJson.object.sha;
      const commitUrl = `${API_ROOT}/repos/${owner}/${repo}/git/commits/${commitSha}`;
      const commitResp = await fetchWithRetry(commitUrl, {}, cacheKey);
      if (commitResp.ok) {
        const commitJson = await commitResp.json();
        if (commitJson && commitJson.tree && commitJson.tree.sha) {
          treeSha = commitJson.tree.sha;
        }
      }
    }
  }

  const treeUrl = `${API_ROOT}/repos/${owner}/${repo}/git/trees/${treeSha}?recursive=1`;
  const treeResp = await fetchWithRetry(treeUrl, {}, cacheKey);
  if (!treeResp.ok) {
    throw new Error("Failed to fetch repo tree.");
  }
  const treeJson = await treeResp.json();
  const entries = (treeJson.tree || []).map((item) => ({
    path: item.path,
    type: item.type === "tree" ? "tree" : "blob",
    sha: item.sha,
  }));

  await Cache.set(cacheKey, entries);
  return entries;
}

/**
 * Converts a GitHub tree array into a VFS snapshot array suitable for cc_init_vfs. [file:2]
 *
 * @param {Array<{ path: string, type: string, sha: string }>} tree
 */
export function treeToVfsSnapshot(tree) {
  const files = [];

  for (const item of tree) {
    if (item.type === "blob") {
      files.push({
        path: item.path,
        content: "", // lazily fetched later
        sha: item.sha,
        is_dir: false,
      });
    } else if (item.type === "tree") {
      files.push({
        path: item.path,
        content: "",
        sha: item.sha || "",
        is_dir: true,
      });
    }
  }

  return files;
}

/**
 * Fetches file content (decoded text) via the Contents API, with caching. [file:2]
 *
 * @param {string} owner
 * @param {string} repo
 * @param {string} path
 * @returns {Promise<string>}
 */
export async function fetchFileContent(owner, repo, path) {
  const cacheKey = `file:${owner}/${repo}:${path}`;
  const cached = await Cache.get(cacheKey);
  if (cached && typeof cached.content === "string") {
    return cached.content;
  }

  const url = `${API_ROOT}/repos/${owner}/${repo}/contents/${encodeURIComponent(
    path
  )}`;
  const resp = await fetchWithRetry(url, {}, cacheKey);
  if (!resp.ok) {
    throw new Error(`Failed to fetch file content for ${path}.`);
  }
  const json = await resp.json();
  const encoded = json.content || "";
  const decoded = decodeBase64(encoded);

  await Cache.set(cacheKey, {
    content: decoded,
    sha: json.sha || "",
    ts: Date.now(),
  });

  return decoded;
}

/**
 * Writes file content back to GitHub using the Contents API (PUT). [file:2]
 * Includes automatic SHA mismatch retry logic.
 *
 * @param {string} owner
 * @param {string} repo
 * @param {string} path
 * @param {string} content
 * @param {string} sha current file SHA or empty for create
 * @param {string} message commit message
 * @param {string} token optional GitHub token for authenticated writes
 * @returns {Promise<{ ok: boolean, sha?: string }>}
 */
export async function writeFile(owner, repo, path, content, sha, message, token) {
  const url = `${API_ROOT}/repos/${owner}/${repo}/contents/${encodeURIComponent(
    path
  )}`;
  const body = {
    message: message || `Update ${path} via Code-Command`,
    content: encodeBase64(content),
  };
  if (sha) {
    body.sha = sha;
  }

  const headers = {
    "Content-Type": "application/json",
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  try {
    const resp = await fetch(url, {
      method: "PUT",
      headers,
      body: JSON.stringify(body),
    });

    // Handle SHA mismatch - refetch latest SHA and retry once
    if (!resp.ok) {
      const errorText = await resp.text();
      if (resp.status === 422 && errorText.includes("SHA")) {
        console.log(`[writeFile] SHA mismatch, refetching latest SHA for ${path}`);
        // Refetch the file to get the latest SHA
        const latestContent = await fetchFileContent(owner, repo, path);
        const cacheKey = `file:${owner}/${repo}:${path}`;
        const cached = await Cache.get(cacheKey);
        const latestSha = cached?.sha || sha;
        
        // Retry with the new SHA
        body.sha = latestSha;
        const retryResp = await fetch(url, {
          method: "PUT",
          headers,
          body: JSON.stringify(body),
        });

        if (!retryResp.ok) {
          const retryText = await retryResp.text();
          throw new Error(`Failed to write file after SHA retry: ${retryText}`);
        }

        const json = await retryResp.json();
        const newSha = json && json.content && typeof json.content.sha === "string"
          ? json.content.sha
          : "";

        await Cache.set(cacheKey, {
          content,
          sha: newSha,
          ts: Date.now(),
        });

        return { ok: true, sha: newSha };
      }

      throw new Error(`Failed to write file: ${errorText}`);
    }

    const json = await resp.json();
    const newSha =
      json && json.content && typeof json.content.sha === "string"
        ? json.content.sha
        : "";

    const cacheKey = `file:${owner}/${repo}:${path}`;
    await Cache.set(cacheKey, {
      content,
      sha: newSha,
      ts: Date.now(),
    });

    return { ok: true, sha: newSha };
  } catch (error) {
    // Record failure in circuit breaker
    const endpoint = 'contents';
    circuitBreaker.recordFailure(endpoint);
    throw error;
  }
}

/* ---------- WASM Bridge Helpers (used by vfs.rs externs) ---------- */ [file:2]

/**
 * Called from WASM to fetch Base64 content for a file path. [file:2]
 * This function name must match the one declared in vfs.rs via wasm_bindgen.
 *
 * @param {string} path
 * @returns {Promise<string>}
 */
export async function wasmFetchFileBase64(path) {
  if (!window.CodeCommandRepo) {
    throw new Error("CodeCommandRepo not configured for wasmFetchFileBase64.");
  }
  const { owner, repo } = window.CodeCommandRepo;
  const content = await fetchFileContent(owner, repo, path);
  return encodeBase64(content);
}

/**
 * Called from WASM to write Base64 content for a file path. [file:2]
 *
 * @param {string} path
 * @param {string} contentBase64
 * @param {string} sha
 * @returns {Promise<boolean>}
 */
export async function wasmWriteFileBase64(path, contentBase64, sha) {
  if (!window.CodeCommandRepo) {
    throw new Error("CodeCommandRepo not configured for wasmWriteFileBase64.");
  }
  const { owner, repo, token } = window.CodeCommandRepo;
  const content = decodeBase64(contentBase64);
  const result = await writeFile(owner, repo, path, content, sha, "", token);
  return !!result.ok;
}

/* ---------- Base64 Helpers (browser-native) ---------- */ [file:2]

function encodeBase64(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function decodeBase64(encoded) {
  const clean = (encoded || "").replace(/\n/g, "");
  const binary = atob(clean);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new TextDecoder().decode(bytes);
}
