// FILE: ./js/app/github/cache.js

// Lightweight IndexedDB-based cache for Code-Command. Stores repository data
// (trees, file contents, SHAs) to reduce GitHub API calls and mitigate rate limits. [file:2]

const DB_NAME = "CodeCommandCache";
const DB_VERSION = 1;
const STORE_NAME = "cache";

let dbPromise = null;

/**
 * Opens (or upgrades) the IndexedDB database. [file:2]
 * @returns {Promise<IDBDatabase>}
 */
function openDb() {
  if (dbPromise) return dbPromise;

  dbPromise = new Promise((resolve, reject) => {
    if (!("indexedDB" in window)) {
      // Fallback: no caching available. [file:2]
      resolve(null);
      return;
    }

    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: "key" });
      }
    };

    request.onsuccess = () => {
      resolve(request.result);
    };

    request.onerror = () => {
      reject(request.error);
    };
  });

  return dbPromise;
}

/**
 * Retrieves a cached value by key, or null if not found. [file:2]
 * @param {string} key
 * @returns {Promise<any|null>}
 */
export async function get(key) {
  const db = await openDb();
  if (!db) return null;

  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readonly");
    const store = tx.objectStore(STORE_NAME);
    const req = store.get(key);

    req.onsuccess = () => {
      const row = req.result;
      resolve(row ? row.value : null);
    };
    req.onerror = () => {
      reject(req.error);
    };
  });
}

/**
 * Stores a value under the given key. [file:2]
 * @param {string} key
 * @param {any} value
 * @returns {Promise<void>}
 */
export async function set(key, value) {
  const db = await openDb();
  if (!db) return;

  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readwrite");
    const store = tx.objectStore(STORE_NAME);
    const entry = {
      key,
      value,
      ts: Date.now(),
    };
    const req = store.put(entry);

    req.onsuccess = () => resolve();
    req.onerror = () => reject(req.error);
  });
}

/**
 * Clears all cached entries. [file:2]
 * @returns {Promise<void>}
 */
export async function clear() {
  const db = await openDb();
  if (!db) return;

  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readwrite");
    const store = tx.objectStore(STORE_NAME);
    const req = store.clear();

    req.onsuccess = () => resolve();
    req.onerror = () => reject(req.error);
  });
}
