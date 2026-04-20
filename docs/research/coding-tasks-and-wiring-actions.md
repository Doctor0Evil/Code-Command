# FILE docs/research/coding-tasks-and-wiring-actions.md
#
# Coding-Tasks and Wiring-Actions Specification for Code-Command
#
# This document defines the concrete, reusable task families and SITQ payload
# patterns that enable "one SITQ, many effects" execution in Code-Command.
#
# CC-Tags: CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV, CC-NAV

---

## 1. High-Level Task Families

Code-Command organizes work into two complementary categories:

- **Coding-Tasks**: One SITQ payload that does real work across the repo in a single execution
- **Wiring-Actions**: Anything that changes how engine components, paths, or policies are connected

### 1.1 Engine & Wiring Introspection Tasks

**Purpose**: Read engine identity, VFS state, and wiring graphs; emit ResearchObjects or wiring reports without mutating the repo.

**CC-Tags**: `CC-FILE`, `CC-PATH`, `CC-SOV`, `CC-NAV`

**Use Cases**:
- Generate current wiring graph from Rust module scans
- Export VFS snapshot metadata
- Produce engine identity reports (component versions, handles)

### 1.2 Policy & Invariant Alignment Tasks

**Purpose**: Scan for CC-tag violations (CC-FILE, CC-FULL, CC-DEEP, CC-SOV, CC-NAV, CC-ZERO), then propose or apply rewrites via SITQ.

**CC-Tags**: `CC-FILE`, `CC-LANG`, `CC-FULL`, `CC-PATH`, `CC-DEEP`, `CC-SOV`, `CC-VOL`, `CC-CRATE`

**Use Cases**:
- Inject missing FILE headers
- Detect and remove excerpt markers ("...", "omitted", "rest of code")
- Validate language compliance (.rs, .js, .cpp, .h, .aln, .md only)
- Check for prohibited external crates

### 1.3 VFS & Path Wiring Tasks

**Purpose**: Normalize, deepen, or relocate files to obey CC-PATH/CC-DEEP using cc-vfs plus PathCanonicalizer rules.

**CC-Tags**: `CC-PATH`, `CC-DEEP`, `CC-FILE`

**Use Cases**:
- Canonicalize file paths across the repository
- Ensure minimum directory depth (≥3) for cache/research artifacts
- Relocate files violating path conventions

### 1.4 Token-Walker / Validator Research Tasks

**Purpose**: Run cc-token-walker benchmarks (10k-line target, scan-profiles), gather metrics, write them into ResearchObjects.

**CC-Tags**: `CC-VOL`, `CC-CRATE`, `CC-NAV`, `CC-SOV`

**Use Cases**:
- Symbol counting benchmarks (Fn, Struct, Class, Mod)
- Import line analysis for sovereignty checks
- Navigation candidate detection
- Volume validation against Vmin thresholds

### 1.5 Connector & Cache Wiring Tasks

**Purpose**: Validate connectors/*adapter.js and .cc-cache layout vs CC-DEEP/CC-SOV; optionally create missing adapter stubs or cache folders.

**CC-Tags**: `CC-DEEP`, `CC-SOV`, `CC-FILE`, `CC-FULL`

**Use Cases**:
- Scaffold new platform adapters (`connectors/<platform>/adapter.js`)
- Validate adapter contract exports (`init`, `fetchRepo`, `sendResult`)
- Create `.cc-cache` subdirectories with proper depth

---

## 2. Concrete SITQ Coding-Task Payloads

### Task A: Normalize Paths and Inject FILE Headers

**Goal**: Enforce CC-FILE, CC-PATH, CC-DEEP on a set of files by injecting headers and normalizing destination paths, without touching semantics.

**Inputs**:
- List of logical paths (e.g., `coreengine/src/lib.rs`, `js/app/main.js`)
- Current contents and SHAs from `ccreadfile` or VFS snapshot

**SITQ Payload Pattern**:

```aln
version 1.0
profile github
tasks
  - kind validateonly
    path coreengine/src/lib.rs
    content <current lib.rs>
    sha <sha>
    tags [CC-FILE, CC-PATH, CC-DEEP, CC-LANG, CC-FULL, CC-SOV]
  - kind writefile
    path coreengine/src/lib.rs
    content <rewritten-with-FILE-header-and-normalized-path>
    sha <sha>
    tags [CC-FILE, CC-PATH, CC-DEEP, CC-FULL, CC-LANG, CC-SOV]
```

**Semantic Wiring**:
1. Task 1 uses validator Tier-1 checks to locate missing or wrong `FILE .path` headers
2. Host (AI or JS) reads ValidationResult and generates corrected content
3. Task 2 writes corrected content only if all tags pass
4. TaskQueue enforces "no write on validation fail" and rolls back on failure

### Task B: Enforce CC-FULL and CC-VOL on a Module Cluster

**Goal**: Ensure no excerpts nor under-volume files within a subtree like `coreengine/src`; schedule high-volume code generation if under-volume.

**SITQ Payload Pattern**:

```aln
version 1.0
profile memory-only
tasks
  - kind validateonly
    path coreengine/src/validator.rs
    content <current>
    sha <sha>
    tags [CC-FULL, CC-VOL, CC-LANG]
  - kind validateonly
    path coreengine/src/tokenwalker.rs
    content <current>
    sha <sha>
    tags [CC-FULL, CC-VOL, CC-LANG]
```

**Flow**:
1. Validator uses cc-token-walker's symbol counting to check CC-VOL vs Vmin
2. CC-FULL bans "...", "rest of code", "omitted"
3. Second round of tasks can propose additional functions to raise volume

### Task C: Wiring Graph Audit and Repair

**Goal**: Verify runtime wiring (Lib ↔ Vfs ↔ Validator ↔ TaskQueue ↔ Navigator) against `specs/wiring-spec.aln` and surface deviations.

**SITQ Payload Pattern**:

```aln
version 1.0
profile memory-only
tasks
  - kind validateonly
    path specs/wiring-spec.aln
    content <current spec>
    sha <sha>
    tags [CC-FILE, CC-LANG, CC-FULL]
  - kind validateonly
    path .cc-cache/validation-reports/wiring-graph.json
    content <generated graph JSON>
    sha ""
    tags [CC-FULL, CC-PATH, CC-SOV]
```

**Process**:
1. Host-side analysis builds WiringGraph by scanning Rust modules with cc-token-walker
2. `validateonly` SITQ task validates generated wiring-graph.json artifact
3. Separate batch of `writefile` tasks adjust call sites to match spec

### Task D: Blacklist Scan and Contamination Report

**Goal**: Treat blacklist hits (Rust Syn, Tree-Sitter, etc.) as first-class contamination events; block any SITQ write containing them.

**SITQ Payload Pattern**:

```aln
version 1.0
profile github
tasks
  - kind validateonly
    path coreengine/src/new_module.rs
    content <candidate content>
    sha ""
    tags [CC-FULL, CC-SOV, CC-BLACKLIST]
```

**Behavior**:
- Validator attaches ContaminationReport entries inside ValidationResult
- SITQ marks tasks failed and rejects subsequent writes
- Reusable "safety" coding-task prefixed to any potentially contaminated input

### Task E: Connector Adapter Scaffolding

**Goal**: Create missing `connectors/<platform>/adapter.js` files; validate existing ones against adapter contract.

**SITQ Payload Pattern**:

```aln
tasks
  - kind writefile
    path connectors/github/adapter.js
    content <full ES-module file with FILE header and required exports>
    sha ""
    tags [CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV]
  - kind validateonly
    path connectors/github/adapter.js
    content <same>
    sha ""
    tags [CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV]
```

**Adapter Contract**:
- Must export: `init(config)`, `fetchRepo(path?)`, `sendResult(result)`
- Must include FILE header with correct path
- Must be complete (no excerpts) per CC-FULL

---

## 3. Wiring-Actions in Rust (Engine Side)

### 3.1 WiringManifest: Centralized Engine Wiring

The **WiringManifest** is the authoritative description of how core components are instantiated and connected.

**Location**: `.core/engine/src/wiring.rs`

**Contract**:
- Owns one `Vfs`, one `Validator`, one `Navigator`, one `TaskQueue`
- `lib.rs` exposes WASM exports delegating to manifest methods
- Enforces: "all writes through TaskQueue", "all navigation through Navigator"

**Key Structures**:
```rust
pub struct EngineGraph {
    pub vfs: Vfs,
    pub validator: Validator,
    pub token_walker: CcTokenWalker,
    pub navigator: Navigator,
    pub task_queue: TaskQueue,
    pub engine_id: &'static str,
    pub vfs_id: &'static str,
}

pub struct WiringManifest<'a> {
    pub vfs: &'a mut Vfs,
    pub validator: &'a mut Validator,
    pub token_walker: &'a mut CcTokenWalker,
    pub navigator: &'a mut Navigator,
    pub task_queue: &'a mut TaskQueue,
    pub engine_id: &'a str,
    pub vfs_id: &'a str,
}
```

### 3.2 DeepWalker / Navigator: CC-NAV Wiring

DeepWalker is the sovereign directory walker that Navigator uses.

**Requirements**:
- Native builds: use `std::fs::read_dir` recursively (no `walkdir` crate)
- WASM builds: call Vfs `list` and treat VFS tree as only filesystem

**Location**: `.core/engine/src/navigator.rs`, `.core/engine/src/tokenwalker.rs`

### 3.3 Validator + cc-token-walker Wiring

Tiered validator calls cc-token-walker as Tier-2 structural engine.

**Wiring Actions**:
- Refactor inline symbol/import scanning into dedicated `tokenwalker` module
- Add ScanProfile bitmask construction for multi-tag efficiency
- Preserve determinism of ValidationResult
- Integrate blacklist so contamination reports are hard failures

**Location**: `.core/engine/src/validator.rs`, `.core/engine/src/tokenwalker.rs`

---

## 4. Wiring-Actions in JS (Host / Connector Side)

### 4.1 js/app/main.js

**Responsibilities**:
- Boots WASM (`initWasm`), creates Monaco editor, FileTree, OutputPanel
- Wires header controls to `loadRepository` and `runCurrentTask`
- Calls `ccinitvfs`, `ccexecutetask`, `ccvalidatecode`
- Must never bypass CCAPI by calling GitHub API directly

### 4.2 connectors/<platform>/adapter.js

**Contract Exports**:
- `init(config)`: Initialize with platform credentials
- `fetchRepo(path?)`: Fetch repository as VFS-SNAPSHOT-1 JSON
- `sendResult(result)`: Commit results back to platform

**Location**: `connectors/github/adapter.js` (reference implementation)

### 4.3 .cc-cache Directory Logic

**Structure** (depth ≥ 3):
```
.cc-cache/
├── vfs-snapshots/     # VFS-SNAPSHOT-1 JSON files
├── symbol-tables/     # cc-token-walker outputs
└── validation-reports/ # ValidationResult JSON, wiring graphs
```

**JS Wiring-Actions**:
- Stash snapshots with unique IDs for later retrieval
- Resolve cached items by ID for reuse in subsequent tasks

---

## 5. Single-Iteration Behaviors

The core invariant: **"one SITQ, many effects"**

A single `ccexecutetask` call can:
- Create/update multiple files across Rust/JS/ALN trees
- Validate each file with full tag set
- Abort and roll back ALL writes if ANY validation fails

**Design Rules**:
1. Each Task includes all invariants (tags) that must hold
2. Engine (not AI) is final gate for validation
3. Wiring changes require lockstep updates to:
   - `.core/engine/src/wiring.rs` (or related modules)
   - `specs/wiring-spec.aln` (or `engine.aln`, `cc-policy.aln`)

---

## 6. Quick Reference: CC-Tag Matrix

| Tag | Purpose | Tier | Checked By |
|-----|---------|------|------------|
| CC-FILE | FILE header presence/path | 1 | String scan |
| CC-LANG | Sovereign language only | 1 | Extension check |
| CC-FULL | No excerpts/omissions | 1 | String scan |
| CC-PATH | Path integrity | 1 | Path canonicalizer |
| CC-DEEP | Minimum directory depth | 1 | Path depth check |
| CC-SOV | No external crates | 2 | TokenWalker imports |
| CC-NAV | Navigation detection | 2 | TokenWalker nav candidates |
| CC-VOL | Minimum symbol volume | 2 | TokenWalker symbol count |
| CC-CRATE | New symbol tracking | 2 | TokenWalker delta |
| CC-ZERO | No env vars, no disk IO | 1 | Static analysis |

---

## 6. AI-Chat Command Grammar

Code-Command exposes a **line-oriented command language** that bridges natural-language AI prompts and SITQ payloads. This grammar lives in docs, in the chat connector, and as a JS helper (`js/app/commands/ai-command-compiler.js`). [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/collection_f23499fa-f1b8-4d9d-a78b-1d96ca262e69/a0605186-d167-45a2-9137-4f051ff9a7f0/code-command-research-agenda-v-K6mH4eczQ2uyz1oqq3WVkg.md)

### 6.1. Command Structure

- One command per line, tokens separated by spaces.
- Optional `[key=value]` blocks for options.
- Comments start with `#`.
- Optional `VERSION 1` header for future compatibility.

### 6.2. Core Commands (Tasks A–E)

| Command | Description | Required Args | Options |
|---------|-------------|---------------|---------|
| `TASK_A <path>` | Normalize paths & inject FILE headers | path (directory) | — |
| `TASK_B <path> [Vmin=N]` | Enforce CC-FULL & CC-VOL | path (directory) | Vmin (default from policy) |
| `TASK_C <path>` | Wiring graph audit | path (module root) | — |
| `TASK_D <path>` | Blacklist scan & contamination report | path (directory) | — |
| `TASK_E <platform>` | Connector adapter scaffolding | platform (e.g., "github") | — |

### 6.3. Example Command Block

```text
VERSION 1
# Heal FILE headers and paths for core engine
TASK_A coreengine/src

# Enforce volume + no excerpts for Rust + JS
TASK_B coreengine/src [Vmin=8]
TASK_B js/app [Vmin=4]

# Ensure GitHub connector is present and valid
TASK_E github
```

### 6.4. Compilation Flow

1. **AI-chat** generates command block from natural language.
2. **Command compiler** (`ai-command-compiler.js`) parses commands → Task objects.
3. **SITQ envelope** wraps tasks with `version: "1.0"`, `profile: "github"`.
4. **Engine** executes via `ccexecutetask`, returns `TaskReport`.
5. **ResearchObjects** emitted to `.cc-cache/validation-reports/`.

### 6.5. Tag Presets

Each Task type has predefined CC-tag sets from `cc-policy.aln`:

- **TASK_A**: `[CC-FILE, CC-PATH, CC-DEEP, CC-LANG, CC-FULL, CC-SOV]`
- **TASK_B**: `[CC-FULL, CC-VOL, CC-LANG]` (+ optional writefile with CC-SOV)
- **TASK_C**: `[CC-FULL, CC-PATH, CC-SOV, CC-DEEP]`
- **TASK_D**: `[CC-FULL, CC-LANG]` + blacklist ScanProfile
- **TASK_E**: `[CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV]`

See `js/app/commands/ai-command-compiler.js` for the canonical implementation.

---

## 7. Implementation Checklist

- [x] Create `connectors/github/adapter.js` with full contract
- [x] Create `.cc-cache/` directory structure
- [x] Implement Task A–E payload generator in JS host (`js/app/commands/ai-command-compiler.js`)
- [ ] Implement Task B volume checker integration with cc-token-walker
- [ ] Wire Task C wiring-graph generation to cc-token-walker
- [ ] Integrate Task D blacklist scan into validator pipeline
- [ ] Build Task E adapter scaffolding automation (auto-detect missing platforms)
- [ ] Update `specs/wiring-spec.aln` with connector node
- [ ] Document ResearchObject schema for benchmark outputs
- [ ] Add UI presets in `js/app/main.js` for Tasks A–E
- [ ] Implement ResearchObject emitter to write to `.cc-cache/validation-reports/`
