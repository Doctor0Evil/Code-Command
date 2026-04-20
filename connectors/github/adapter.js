// FILE connectors/github/adapter.js
//
// GitHub connector adapter for Code-Command.
// Implements the standard adapter contract: init, fetchRepo, sendResult.
//
// CC-Tags: CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV
//
// This adapter is responsible for:
// - Initializing GitHub API credentials and configuration
// - Fetching repository tree and file contents as VFS snapshots
// - Sending results (validation reports, ResearchObjects) back to GitHub
//
// Invariants:
// - No direct network IO from engine; all HTTP calls flow through this adapter
// - All file contents are converted to base64 for WASM compatibility
// - VFS snapshots follow VFS-SNAPSHOT-1 schema

/**
 * Initialize the GitHub adapter with configuration.
 * @param {Object} config - Configuration object
 * @param {string} config.token - GitHub personal access token
 * @param {string} config.owner - Repository owner (user or org)
 * @param {string} config.repo - Repository name
 * @param {string} [config.branch] - Branch to operate on (default: main)
 * @returns {Promise<Object>} Adapter instance with fetchRepo and sendResult
 */
export async function init(config) {
  if (!config || !config.token || !config.owner || !config.repo) {
    throw new Error('GitHub adapter requires token, owner, and repo');
  }

  const baseUrl = 'https://api.github.com';
  const branch = config.branch || 'main';

  const headers = {
    'Authorization': `token ${config.token}`,
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'Code-Command-Adapter/1.0'
  };

  /**
   * Fetch repository tree and contents as a VFS snapshot.
   * @param {string} [path=''] - Optional subpath to fetch
   * @returns {Promise<Object>} VFS snapshot in VFS-SNAPSHOT-1 format
   */
  async function fetchRepo(path = '') {
    const treeUrl = `${baseUrl}/repos/${config.owner}/${config.repo}/git/trees/${branch}?recursive=1`;
    
    const treeResponse = await fetch(treeUrl, { headers });
    if (!treeResponse.ok) {
      throw new Error(`Failed to fetch tree: ${treeResponse.status}`);
    }
    const treeData = await treeResponse.json();

    // Filter to requested path if specified
    let files = treeData.tree || [];
    if (path) {
      const normalizedPath = path.replace(/^\/+/, '');
      files = files.filter(f => f.path.startsWith(normalizedPath));
    }

    // Build VFS snapshot structure
    const vfsSnapshot = {
      version: 'VFS-SNAPSHOT-1',
      root: `${config.owner}/${config.repo}`,
      branch: branch,
      fetchedAt: new Date().toISOString(),
      files: []
    };

    // Fetch content for each file (excluding blobs > 1MB via GitHub API limits)
    for (const item of files) {
      if (item.type === 'blob') {
        try {
          const contentUrl = `${baseUrl}/repos/${config.owner}/${config.repo}/contents/${item.path}?ref=${branch}`;
          const contentResponse = await fetch(contentUrl, { headers });
          if (contentResponse.ok) {
            const contentData = await contentResponse.json();
            vfsSnapshot.files.push({
              path: contentData.path,
              sha: contentData.sha,
              encoding: contentData.encoding, // 'base64'
              content: contentData.content,   // base64-encoded
              size: contentData.size
            });
          }
        } catch (err) {
          // Skip files that fail to fetch (e.g., too large)
          console.warn(`Skipping ${item.path}: ${err.message}`);
        }
      }
    }

    return vfsSnapshot;
  }

  /**
   * Send results (validation reports, ResearchObjects) back to GitHub.
   * @param {Object} result - Result object to commit
   * @param {string} result.path - Destination path in repo
   * @param {string} result.content - File content (UTF-8 string)
   * @param {string} [result.message] - Commit message
   * @param {string} [result.branch] - Target branch (overrides default)
   * @returns {Promise<Object>} GitHub commit response
   */
  async function sendResult(result) {
    if (!result || !result.path || result.content === undefined) {
      throw new Error('sendResult requires path and content');
    }

    const contentUrl = `${baseUrl}/repos/${config.owner}/${config.repo}/contents/${result.path}`;
    const targetBranch = result.branch || branch;

    // Get current SHA if file exists
    let sha = null;
    try {
      const getResponse = await fetch(contentUrl, { headers });
      if (getResponse.ok) {
        const existing = await getResponse.json();
        sha = existing.sha;
      }
    } catch (err) {
      // File doesn't exist yet, that's fine
    }

    // Prepare commit payload
    const payload = {
      message: result.message || `Code-Command: update ${result.path}`,
      content: btoa(unescape(encodeURIComponent(result.content))),
      branch: targetBranch
    };

    if (sha) {
      payload.sha = sha;
    }

    const putResponse = await fetch(contentUrl, {
      method: 'PUT',
      headers,
      body: JSON.stringify(payload)
    });

    if (!putResponse.ok) {
      const errorText = await putResponse.text();
      throw new Error(`Failed to commit ${result.path}: ${putResponse.status} - ${errorText}`);
    }

    return await putResponse.json();
  }

  return {
    config: { owner: config.owner, repo: config.repo, branch },
    fetchRepo,
    sendResult
  };
}

/**
 * Default export matching the adapter contract.
 */
export default { init };
