// FILE: ./js/app/github/api.js

// GitHub REST API wrapper for Code-Command. Uses only the official
// GitHub REST endpoints as a sovereign backend (no other external services). [file:2]

import * as Cache from "./cache.js";

const API_ROOT = "https://api.github.com";

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
  const refResp = await fetch(
    `${API_ROOT}/repos/${owner}/${repo}/git/refs/heads/${encodeURIComponent(
      "main"
    )}`
  );
  let treeSha = sha;
  if (refResp.ok) {
    const refJson = await refResp.json();
    if (refJson && refJson.object && refJson.object.sha) {
      const commitSha = refJson.object.sha;
      const commitResp = await fetch(
        `${API_ROOT}/repos/${owner}/${repo}/git/commits/${commitSha}`
      );
      if (commitResp.ok) {
        const commitJson = await commitResp.json();
        if (commitJson && commitJson.tree && commitJson.tree.sha) {
          treeSha = commitJson.tree.sha;
        }
      }
    }
  }

  const treeResp = await fetch(
    `${API_ROOT}/repos/${owner}/${repo}/git/trees/${treeSha}?recursive=1`
  );
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
  const resp = await fetch(url);
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

  const resp = await fetch(url, {
    method: "PUT",
    headers,
    body: JSON.stringify(body),
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`Failed to write file: ${text}`);
  }

  const json = await resp.json();
  const newSha =
    json && json.content && typeof json.content.sha === "string"
      ? json.content.sha
      : "";

  await Cache.set(`file:${owner}/${repo}:${path}`, {
    content,
    sha: newSha,
    ts: Date.now(),
  });

  return { ok: true, sha: newSha };
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
