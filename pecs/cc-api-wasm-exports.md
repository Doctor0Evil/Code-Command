# CC-API WebAssembly Exports for cc-engine

This document defines the **GitHub-ready, canonical interface** that the `cc-engine` WebAssembly module exposes to the browser runtime. It wraps the sovereign `VirtualFileSystem` (`Vfs`) and connects it to JavaScript, while enforcing the Code-Command invariants such as CC‑ZERO, CC‑FILE, CC‑VOL, CC‑PATH, CC‑DEEP, and CC‑SOV.[file:1][file:3]

All functions described here live in `./core/engine/src/lib.rs` and are exported to JavaScript via `wasm-bindgen` as part of the `cc-engine.wasm` module.[file:1] They form the **CC‑API surface** that the browser UI, GitHub integration layer, and GitHub Actions rely on.

---

## 1. File and module layout

The Rust file that implements these exports is:

```rust
// FILE: ./core/engine/src/lib.rs

#![allow(clippy::unused_unit)]

mod validator;
mod vfs;
mod taskqueue;

use validator::{ValidationRequest, ValidationResult};
use vfs::{Vfs, VirtualFileSystem};
use taskqueue::TaskQueue;

use wasm_bindgen::prelude::*;

// Global VFS instance for the WASM engine.
static mut VFS: Option<Vfs> = None;
```

**Key invariants:**[file:1][file:2]  
The imports are all **relative** (no external runtime crates), and the file path is depth‑3 under `core/engine/src/lib.rs`, which aligns with CC‑ZERO and CC‑DEEP.[file:1][file:2] The single `VFS` static serves as the session-scoped `VirtualFileSystem` instance owned by the WebAssembly module.[file:1]

---

## 2. ccinitvfs — Initialize the Virtual File System

### Signature

```rust
#[wasm_bindgen]
pub fn ccinitvfs(serialized_vfs_json: &str)
```

### Description

`ccinitvfs` seeds the in‑memory `Vfs` instance with a snapshot of repository entries provided by JavaScript.[file:1][file:3] This snapshot is typically derived from the Git Trees and Contents APIs and serialized into a JSON array of file/directory objects.

### Input format

The `serialized_vfs_json` parameter must be a JSON string with the following shape:[file:1][file:3]

```json
[
  {"path": "src/main.rs", "content": "fn main() {}", "sha": "abc123", "isdir": false},
  {"path": "src", "content": "", "sha": "", "isdir": true}
]
```

Each object is converted into a `FileEntry` and inserted into `Vfs.files` with path normalization applied.[file:1]

### Behavior and invariants

Internally, `ccinitvfs` behaves as follows:[file:1][file:3]

```rust
#[wasm_bindgen]
pub fn ccinitvfs(serialized_vfs_json: &str) {
    let vfs = Vfs::from_json(serialized_vfs_json);

    unsafe {
        VFS = Some(vfs);
    }
}
```

- **CC‑PATH / CC‑DEEP**: enforced by `Vfs::from_json`, which calls `normalize_path` on each `path` before insertion and uses depth checks where appropriate.[file:1]  
- **Error handling**: malformed JSON or unexpected structures result in an empty `Vfs`, but never a panic; the caller is expected to validate inputs on the JS side.[file:1][file:3]  
- **State semantics**: replaces any previously initialized `VFS` instance, making the snapshot authoritative for the current session.[file:1]  

This function contains no environment, install, or setup logic, preserving CC‑ZERO.[file:1][file:2]

---

## 3. ccreadfile — Read from cc‑vfs

### Signature

```rust
#[wasm_bindgen]
pub fn ccreadfile(path: &str) -> String
```

### Description

`ccreadfile` returns the current textual content of a file located at the logical repository path `path`.[file:1][file:3] It consults the in‑memory `Vfs` and returns an empty string if the file does not exist, is a directory, or if the VFS has not yet been initialized.[file:1]

### Behavior

Implementation in `lib.rs`:

```rust
#[wasm_bindgen]
pub fn ccreadfile(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.read(path).unwrap_or_default();
        }
    }

    String::new()
}
```

### Invariants

- `VirtualFileSystem::read` normalizes the path via `normalize_path`, enforcing CC‑PATH and stripping malformed segments.[file:1]  
- If the entry at `path` is a directory (`is_dir == true`), `read` returns `None`, and the exported function returns `""`.[file:1][file:3]  
- No network calls occur inside this function; JS remains responsible for any GitHub fallback or cache population.[file:1]  

This behavior matches the CC‑API description for `ccreadfile` in your specification.[file:1][file:3]

---

## 4. ccwritefile — Write into cc‑vfs

### Signature

```rust
#[wasm_bindgen]
pub fn ccwritefile(path: &str, content: &str, sha: &str) -> bool
```

### Description

`ccwritefile` updates or creates a file entry in the in‑memory `Vfs` at `path` with the given `content` and `sha`.[file:1][file:3] It enforces path invariants and signals whether the write is accepted by the in‑memory VFS.

### Behavior

```rust
#[wasm_bindgen]
pub fn ccwritefile(path: &str, content: &str, sha: &str) -> bool {
    unsafe {
        if let Some(vfs) = &mut VFS {
            let ok = vfs.write(path, content, sha);
            return ok;
        }
    }

    false
}
```

### Invariants

- **Path normalization and integrity**: `Vfs::write` calls `normalize_path`, rejecting malformed paths, enforcing CC‑PATH.[file:1]  
- **Depth constraints**: `Vfs::write` ensures CC‑DEEP for core engine modules, requiring depth ≥ 3 for protected paths.[file:1][file:2]  
- **Directory semantics**: writes always produce `is_dir == false` entries; directories remain separate.[file:1]  
- **Backend separation**: this function does not perform network I/O; JS uses the same `path`, `content`, and `sha` to execute a GitHub Contents API PUT or equivalent side effect.[file:1][file:3]  

If the `VFS` is uninitialized or invariants fail, the function returns `false` and JS should treat the write as rejected.[file:1]

---

## 5. cclistdir — List directory entries

### Signature

```rust
#[wasm_bindgen]
pub fn cclistdir(path: &str) -> String
```

### Description

`cclistdir` returns a JSON array of **direct children** under the directory at `path`.[file:1][file:3] This function powers the file‑tree component in the browser and any CC‑NAV‑related traversal logic that works over cc‑vfs.

### Behavior

```rust
#[wasm_bindgen]
pub fn cclistdir(path: &str) -> String {
    unsafe {
        if let Some(vfs) = &VFS {
            return vfs.list(path);
        }
    }

    "[]".to_string()
}
```

### Output format

The returned JSON follows this shape:[file:1]

```json
[
  {"path": "core/engine/src", "isdir": true, "sha": ""},
  {"path": "core/engine/src/lib.rs", "isdir": false, "sha": "abc123"}
]
```

Each object is derived from a `FileEntry` that is a direct child of the normalized `path`, as determined by `is_direct_child_of` in `vfs.rs`.[file:1]

### Invariants

- Only **direct children** are included; grandchildren or deeper descendants are excluded.[file:1]  
- Paths are normalized and never contain backslashes or double slashes, satisfying CC‑PATH.[file:1][file:2]  
- Empty or uninitialized VFS yields `"[]"`.[file:1]

This behavior matches the CC‑API `cclistdir` description used by your specs and UI.[file:1][file:3]

---

## 6. ccvalidatecode — Invariant validation

### Signature

```rust
#[wasm_bindgen]
pub fn ccvalidatecode(code: &str, tags_json: &str) -> JsValue
```

### Description

`ccvalidatecode` runs the custom invariant validator over a single code artifact.[file:1][file:2] It checks the requested CC‑ tags (e.g., `CC-FILE`, `CC-DEEP`) and returns a JSON result summarizing which invariants passed or failed.

### Behavior

```rust
#[wasm_bindgen]
pub fn ccvalidatecode(code: &str, tags_json: &str) -> JsValue {
    let req = ValidationRequest::from_json(code, tags_json);
    let result: ValidationResult = validator::run_validation(req);
    JsValue::from_str(&result.to_json())
}
```

### Input and output

- `code`: the full source text of a file.  
- `tags_json`: JSON array of tag IDs, such as `["CC-FILE","CC-DEEP","CC-ZERO"]`.[file:2][file:3]  

The returned `JsValue` stringifies to JSON of the form:[file:1]

```json
{
  "ok": false,
  "failures": [
    "CC-FILE: Missing FILE header in first lines of file.",
    "CC-DEEP: Path does not satisfy depth 3 after normalization."
  ]
}
```

### Invariants

- Uses only custom, hand‑rolled parsing (`ValidationRequest::from_json`, token walker) and no external crates, satisfying CC‑SOV.[file:1][file:2]  
- Enforces all ten invariants: CC‑VOL, CC‑LANG, CC‑CRATE, CC‑FILE, CC‑FULL, CC‑DEEP, CC‑ZERO, CC‑PATH, CC‑SOV, CC‑NAV.[file:2][file:3]  
- Validators rely on the same `normalize_path` and depth logic as cc‑vfs, ensuring consistent path semantics.[file:1]

---

## 7. ccexecutetask — Single‑Iteration Task Queue

### Signature

```rust
#[wasm_bindgen]
pub fn ccexecutetask(task_json: &str) -> String
```

### Description

`ccexecutetask` executes a **Single‑Iteration Task Queue (SITQ)** payload over the current `Vfs`.[file:1][file:3] It allows multiple file operations and validations to be performed in one call, supporting CC‑VOL by design.

### Behavior

```rust
#[wasm_bindgen]
pub fn ccexecutetask(task_json: &str) -> String {
    let mut queue = TaskQueue::from_json(task_json);

    unsafe {
        if let Some(vfs) = &mut VFS {
            let report = queue.execute(vfs);
            return report.to_json();
        }
    }

    TaskQueue::empty_failure("VFS not initialized.").to_json()
}
```

### Input format

`task_json` encodes a list of operations and optional validations, for example:[file:1]

```json
{
  "tasks": [
    {"kind": "write", "path": "core/engine/src/vfs.rs", "content": "...", "sha": "abc123"},
    {"kind": "validate", "path": "core/engine/src/vfs.rs", "tags": ["CC-FILE","CC-DEEP","CC-PATH"]}
  ]
}
```

`TaskQueue::from_json` is a handwritten parser that adheres to CC‑SOV by avoiding external serialization libraries.[file:1][file:3]

### Output format

The returned JSON report typically includes:[file:1]

```json
{
  "ok": true,
  "operations": [
    {"path": "core/engine/src/vfs.rs", "status": "written"}
  ],
  "validations": [
    {"path": "core/engine/src/vfs.rs", "ok": true, "failures": []}
  ]
}
```

### Invariants

- Uses `Vfs::write` and `ccvalidatecode` internally, so all CC‑PATH, CC‑DEEP, and CC‑tag invariants are enforced for each task.[file:1][file:2]  
- If `VFS` is uninitialized, a structured failure report is returned, not a panic.[file:1][file:3]  
- Executes multiple actions per invocation, aligning with CC‑VOL’s requirement for substantial work per iteration.[file:2]

---

## 8. Integration notes for GitHub and browser runtimes

To integrate this CC‑API with GitHub and the browser, the JS layer (e.g., `./js/app/main.js` and `./js/app/github/api.js`) should:[file:1][file:3]

1. **Fetch repository snapshot** using GitHub Trees and Contents APIs, then assemble a JSON array matching the `ccinitvfs` snapshot format.[file:1][file:3]  
2. **Call `ccinitvfs(snapshot)`** immediately after the WASM module is instantiated to seed cc‑vfs.[file:1]  
3. Use **`ccreadfile`**, **`ccwritefile`**, and **`cclistdir`** for all file interactions in the editor and file tree, keeping the VFS as the single source of truth.[file:1]  
4. Call **`ccvalidatecode`** before committing changes or as part of CI to enforce the CC‑ tags on generated or edited code.[file:2][file:3]  
5. Use **`ccexecutetask`** for higher-level operations where a single user prompt should create, modify, and validate multiple files at once.[file:1]

By adhering to this exported interface, the browser and GitHub Actions can treat `cc-engine.wasm` as a **sovereign, zero‑setup coding agent** that runs directly from the repository with no additional installation steps, in full compliance with your R‑rules and invariant policy surface.[file:1][file:2][file:3]
