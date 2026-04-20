// FILE: ./docs/research/code-command-research-agenda-supplement.md  
// DESTINATION: ./docs/research/code-command-research-agenda-supplement.md  

# Code-Command Research Agenda Supplement  
**Version:** 1.0.0  
**Status:** Active Inquiry  
**Purpose:** This supplement provides an additional 100 research questions, definition requests, detail queries, and objection identifiers focused specifically on **blacklisting capabilities** and **wiring together components** within the Code‑Command repository. It covers precise definitions for directories, filenames, inter‑module connections, and enforcement mechanisms that align with the 10 CC‑ tags and the zero‑setup, sovereign‑stack requirements.

---

## Table of Contents  
1. [Research Questions – Blacklisting & Wiring](#research-questions-blacklisting--wiring) – 40 items  
2. [Definition Requests – Blacklisting & Wiring](#definition-requests-blacklisting--wiring) – 30 items  
3. [Detail Queries – Blacklisting & Wiring](#detail-queries-blacklisting--wiring) – 20 items  
4. [Objection Identifiers – Blacklisting & Wiring](#objection-identifiers-blacklisting--wiring) – 10 items  

---

## Research Questions – Blacklisting & Wiring  

**RQ‑35:** How can a finite‑state automaton scan source files byte‑by‑byte to detect the exact sequence `(*/)` without false positives on legitimate comment closings like `*/`?  

**RQ‑36:** What in‑memory data structure efficiently tracks every external dependency reference (e.g., `use serde_json`, `#include <boost/...>`, `require('axios')`) across all source files to produce a comprehensive `CC‑SOV` violation report?  

**RQ‑37:** How can the agent dynamically build a whitelist of permitted module imports derived solely from files that already exist within the repository’s `src/` tree and carry a valid `CC‑FILE` header?  

**RQ‑38:** What algorithm can traverse the abstract syntax representation (as captured by `cc‑token‑walker`) to identify all function‑call edges between Rust modules and verify that each callee resides in a file that satisfies the depth‑3 core‑module requirement?  

**RQ‑39:** How can a Rust‑based validator detect the presence of `std::env::temp_dir`, `install`, or `setup` tokens in any entry file while allowing them to appear in non‑entry files (e.g., `core/engine/src/vfs.rs` may legitimately use `std::fs`)?  

**RQ‑40:** What is the minimum viable set of banned patterns for `CC‑FULL` beyond ellipsis and `omitted`, and how can those patterns be stored in an `ALN` block so that the validator reads them at runtime without recompilation?  

**RQ‑41:** How can the agent wire the output of the `cc‑token‑walker` symbol collector directly into the `CC‑CRATE` comparison logic to determine whether a new conversation delta contains at least one novel `struct`, `fn`, or `mod` declaration?  

**RQ‑42:** What is the exact sequence of Rust function calls that connects a user‑provided prompt (e.g., “write a new connector”) to the creation of a `writefile` task in the SITQ, including validation, path normalization, and `CC‑FILE` header insertion?  

**RQ‑43:** How can the JavaScript frontend `js/app/main.js` initialize the WASM module, fetch a repository snapshot via `js/app/github-api.js`, and pass the resulting `cc‑vfs` snapshot to the Rust core without any intermediate state that could violate `CC‑ZERO`?  

**RQ‑44:** What is the optimal event‑dispatch mechanism in pure JavaScript (no external libraries) that allows the file‑tree component to refresh when the Rust core completes a `writefile` task and emits a `cc‑vfs` change notification?  

**RQ‑45:** How can the agent implement a bidirectional communication channel between Rust and C++ using only `extern "C"` and a pre‑compiled static library, such that path‑sanitization calls from Rust are forwarded to `cpp/fallback/path-sanitizer.cpp` and results are returned without heap allocation on the C++ side?  

**RQ‑46:** What deterministic naming scheme should be applied to the `.cc‑cache` subdirectory to avoid collisions when multiple versions of the same repository are loaded in different browser tabs or AI‑Chat sessions?  

**RQ‑47:** How can the Rust core detect that a JavaScript file attempts to access `window`, `document`, or `localStorage` and treat those accesses as a `CC‑SOV` violation (since they imply non‑sovereign browser APIs)?  

**RQ‑48:** What is the most compact serialization format for a `ResearchObject` that bundles a `cc‑vfs` snapshot, a set of `CCTokenWalker` metrics, and a `ValidationResult` list, suitable for transmission between the AI‑Chat platform and the Code‑Command backend?  

**RQ‑49:** How can the agent wire the `Blacklist(*/)` detector into the `TaskQueue` such that any task containing a contamination marker is rejected before any file write occurs, and a professional report is appended to the `ValidationResult` stream?  

**RQ‑50:** What is the precise algorithm for recursively scanning all `.aln` and `.md` policy files in the repository to extract active `CC‑` tags and merge them with command‑line overrides provided via the `SITQ` task payload?  

**RQ‑51:** How can a Rust module named `core/engine/src/wiring.rs` expose a single public function `fn connect_all() -> Result<EngineGraph, WiringError>` that instantiates and links `Validator`, `Vfs`, `Navigator`, `TaskQueue`, and `TokenWalker` with explicit ownership transfer?  

**RQ‑52:** What is the exact directory layout for a “connector” that adapts Code‑Command to a specific AI‑Chat platform (e.g., `connectors/chatgpt/`), and what mandatory files (`manifest.aln`, `adapter.js`, `policy.aln`) must be present to satisfy `CC‑DEEP`?  

**RQ‑53:** How can the agent ensure that any file created under `connectors/` automatically receives a `CC‑FILE` header and a `CC‑LANG`‑compliant extension, and how is this enforced differently from core engine files?  

**RQ‑54:** What is the minimal set of exported WASM functions (`cc_init_vfs`, `cc_validate_code`, `cc_execute_task`, etc.) required for the JavaScript frontend to perform all coding‑agent operations without any additional Rust‑side logic?  

**RQ‑55:** How can the Rust validator maintain an in‑memory “import graph” that maps each file to the set of modules it depends on, using only the information provided by `cc‑token‑walker` and without constructing a full Rust `mod` hierarchy?  

**RQ‑56:** What is the most efficient way to store and query the canonical set of banned crates (e.g., `serde`, `tokio`, `reqwest`) such that `CC‑SOV` checks run in `O(1)` per `use` statement found by the token walker?  

**RQ‑57:** How can the agent wire the `navigator.rs` module’s `walkdir` function to a caching layer that stores file metadata (`size`, `sha`, `last_modified`) in the `.cc‑cache` directory and invalidates the cache when the underlying GitHub repository changes?  

**RQ‑58:** What is the precise format of the “professional report” emitted when a `(*/)` blacklist item is detected, and how does this report differ from a standard `ValidationResult` entry in terms of severity and required user action?  

**RQ‑59:** How can the agent implement a custom `PathCanonicalizer` that resolves `./` and `../` segments using only the `cc‑vfs` in‑memory snapshot, never touching the host OS filesystem, thereby maintaining `CC‑ZERO` compliance even during path normalization?  

**RQ‑60:** What is the algorithm for determining whether a given file path belongs to a “core module” (depth ≥ 3) versus a “leaf utility” (depth < 3) and how is this distinction communicated from `invariants.aln` to the `CC‑DEEP` validator?  

**RQ‑61:** How can the JavaScript frontend dynamically load and display the contents of a `cc‑vfs` snapshot as a file tree, with real‑time updates when `writefile` or `deletefile` tasks complete, using only vanilla DOM manipulation?  

**RQ‑62:** What is the exact sequence of `postMessage` calls (or equivalent) required for the agent to run inside a Web Worker while keeping the `WASM` module on the main thread, and how does this affect `CC‑ZERO` constraints?  

**RQ‑63:** How can the agent wire a custom error‑reporting channel that captures all Rust `panic!` outputs (only for programmer errors) and converts them into structured `ValidationResult` entries with a `CC‑PANIC` tag?  

**RQ‑64:** What is the most compact representation of a `ScanProfile` bitmask that encodes the active set of `CC‑` tags, `LanguageHint`, and `Blacklist` patterns, and how is this bitmask passed from the policy parser to `cc‑token‑walker`?  

**RQ‑65:** How can the Rust core serialize a `cc‑vfs` snapshot to a `Uint8Array` for efficient transfer across the WASM boundary, and what is the corresponding JavaScript deserialization routine?  

**RQ‑66:** What is the algorithm for merging two `cc‑vfs` snapshots (e.g., the base repository snapshot plus a set of unsaved local edits) without losing change history or violating `CC‑PATH` invariants?  

**RQ‑67:** How can the agent detect circular dependencies between Rust modules using only the import graph constructed by `cc‑token‑walker`, and how should such cycles be reported under a `CC‑CYCLE` tag?  

**RQ‑68:** What is the exact file‑naming convention for policy files that distinguishes between mandatory invariants (e.g., `invariants.aln`), per‑directory overrides (e.g., `connectors/.ccpolicy.aln`), and user‑supplied configuration?  

**RQ‑69:** How can the JavaScript `github-api.js` module implement exponential backoff and request queuing to stay within GitHub API rate limits without requiring any user configuration or external libraries?  

**RQ‑70:** What is the most efficient way to diff two `CCTokenWalker` symbol sets to determine the exact set of new declarations introduced in a conversation delta, and how is this diff stored for `CC‑CRATE` validation?  

**RQ‑71:** How can the agent wire the `Blacklist(*/)` detector to also scan `ALN` policy files and Markdown documentation, ensuring that no contamination ever appears in any repository artifact visible to users?  

**RQ‑72:** What is the precise directory structure under `specs/` that holds formal specifications for each `CC‑` tag, and how does the Rust validator locate and parse these spec files when generating compliance reports?  

**RQ‑73:** How can the `TaskQueue` module implement a “dry‑run” mode that simulates all file writes and returns the expected post‑task `cc‑vfs` snapshot without actually modifying any in‑memory state?  

**RQ‑74:** What is the minimal set of C++ functions that must be compiled into `cpp/fallback/libccpath.a` to support path sanitization, and how is this static library linked into the final WASM binary without using `bindgen`?  

---

## Definition Requests – Blacklisting & Wiring  

**DR‑34:** Define the exact syntax for an `ALN` blacklist entry that specifies a banned token, the language context in which it applies, and the severity level (e.g., `block`, `warn`, `report`).  

**DR‑35:** Define the term “professional report” in the context of `Blacklist(*/)` detection: list the required sections (e.g., `Incident ID`, `Timestamp`, `Location`, `Contaminated Content`, `Recommended Action`) and the output format (JSON, Markdown, plain text).  

**DR‑36:** Define the exact shape of a `WiringManifest` struct that describes how `Validator`, `Vfs`, `Navigator`, `TaskQueue`, and `TokenWalker` are instantiated and connected, including any shared references (e.g., `Arc<Vfs>`) or channels.  

**DR‑37:** Define the internal representation of a `ModuleGraph` that maps file paths to lists of imported modules, including the edge type (`use`, `require`, `#include`) and the source location (line, column).  

**DR‑38:** Define the exact filename and extension for a “connector adapter” file, e.g., `connectors/[platform]/adapter.js`, and the required exported functions (`init`, `fetchRepo`, `sendResult`).  

**DR‑39:** Define the term “sovereign import” as used in `CC‑SOV`: an import that references a file within the repository’s `src/` tree and is allowed; contrast with “external import” which references a crate, package, or header not authored within Code‑Command.  

**DR‑40:** Define the exact directory layout for the `.cc‑cache` folder, including subdirectories for `vfs_snapshots/`, `symbol_tables/`, and `validation_reports/`, and specify which of these are subject to `CC‑DEEP` depth requirements.  

**DR‑41:** Define the precise grammar of a `CC‑FILE` header for each supported language (Rust, JavaScript, C++, Markdown, ALN), including the exact comment delimiter and the required whitespace around the `FILE` keyword.  

**DR‑42:** Define the term “entry file” as it relates to `CC‑ZERO`: list the specific files (e.g., `src/lib.rs`, `js/app/main.js`, `index.html`) that are subject to stricter banned‑token checks.  

**DR‑43:** Define the exact JSON schema for a `Task` object in the SITQ, including the `kind` field values (`writefile`, `deletefile`, `validateonly`), the required `path`, `content`, and `sha` fields, and the optional `tags` array.  

**DR‑44:** Define the internal data structure for a `ContaminationReport` that is appended to `ValidationResult` when a `(*/)` blacklist item is found, including fields for `pattern`, `exact_match`, `surrounding_context`, and `severity`.  

**DR‑45:** Define the term “wiring graph” as the set of explicit function‑call edges between the major modules of the Code‑Command engine, and specify how this graph is validated against a reference specification (e.g., `specs/wiring‑spec.aln`).  

**DR‑46:** Define the exact set of environment variables that the Rust core is permitted to read under `CC‑ZERO` (currently none), and provide a justification for any future exceptions.  

**DR‑47:** Define the precise byte‑level format of the `ScanProfile` bitmask, allocating specific bits for each of the 10 `CC‑` tags, the `Blacklist` flag, and the `LanguageHint` enumeration.  

**DR‑48:** Define the term “depth‑3 core module” by providing a regular expression that matches allowed core paths (e.g., `src/[^/]+/[^/]+/[^/]+\.rs`) and excludes leaf utilities.  

**DR‑49:** Define the exact API of the `DeepWalker` iterator, including its associated `Item` type (e.g., `(PathBuf, VfsEntry)`) and the methods `new(root: &Path, max_depth: usize) -> Self` and `filter_extensions(exts: &[&str]) -> Self`.  

**DR‑50:** Define the term “policy override” as the mechanism by which a `.ccpolicy.aln` file in a subdirectory can modify the active set of `CC‑` tags or adjust thresholds (e.g., `V_min`) for that subtree only.  

**DR‑51:** Define the exact schema for a `ResearchObject` that combines a `cc‑vfs` snapshot, a `CCTokenWalker` metrics report, and a list of `ValidationResult` entries, including versioning and a checksum for integrity.  

**DR‑52:** Define the internal representation of a `Symbol` as produced by `cc‑token‑walker`, including fields for `name`, `kind` (`Fn`, `Struct`, `Class`, `Mod`), `language`, `file_path`, and `location` (line, column).  

**DR‑53:** Define the term “connector lifecycle” from the perspective of the Rust core: what `cc‑API` functions are called in what order to initialize a platform‑specific session, fetch a repository, run a task, and return results.  

**DR‑54:** Define the exact format of the `cc‑vfs` snapshot JSON, including the top‑level `version` field, the `files` array, and the required `path`, `content`, `sha`, and `is_dir` fields for each entry.  

**DR‑55:** Define the term “wiring efficiency” as a measurable metric: the number of cross‑module function calls required to process a single `validateonly` task for a 1000‑line Rust file.  

**DR‑56:** Define the exact set of C++ header files that are permitted under `CC‑LANG` and `CC‑SOV` (only `.h` files authored within the repository; no system headers like `<iostream>` unless absolutely justified).  

**DR‑57:** Define the internal representation of a `BlacklistRule` that can be loaded from an `ALN` file, including fields for `pattern` (string), `is_regex` (boolean), `languages` (array), and `action` (`block` or `warn`).  

**DR‑58:** Define the term “mount” and “unmount” in the context of the `cc‑vfs`: mounting means calling `cc_init_vfs(snapshot_json)` to set the active virtual filesystem; unmounting means discarding the current snapshot and reverting to an empty state.  

**DR‑59:** Define the exact filename and location of the “wiring specification” document that describes how all Rust modules are connected, e.g., `specs/wiring‑spec.aln`.  

**DR‑60:** Define the precise grammar for an `ALN` policy block that declares a `CC‑` tag, including the `tag`, `status`, `description`, `machine_rule`, and `pseudocode` fields.  

**DR‑61:** Define the internal data structure for a `ValidationResult` entry, including `tag` (e.g., `CC‑FILE`), `passed` (boolean), `message`, `path`, `line`, `column`, and `severity` (`error`, `warning`, `info`).  

**DR‑62:** Define the term “discovery logic” as implemented in `navigator.rs`: the algorithm that traverses a directory tree, applies `CC‑LANG` and `CC‑DEEP` filters, and returns a list of `ResearchObject` candidates.  

**DR‑63:** Define the exact API of the `JsonBuilder` struct, including method signatures for `new()`, `start_object()`, `key(&str)`, `value_string(&str)`, `value_number(i64)`, `value_bool(bool)`, `end_object()`, `start_array()`, `end_array()`, and `build() -> String`.  

---

## Detail Queries – Blacklisting & Wiring  

**DQ‑24:** Provide a step‑by‑step description of how the `Blacklist(*/)` detector is integrated into the `validate_code` function, from the moment a file’s content is loaded to the point where a `ContaminationReport` is appended to the result.  

**DQ‑25:** Detail the exact sequence of Rust function calls that occurs when the JavaScript frontend invokes `cc_execute_task` with a `writefile` task payload, including validation, header injection, `cc‑vfs` update, and cache invalidation.  

**DQ‑26:** Provide a complete listing of all banned crate names that must be rejected by `CC‑SOV`, including common serialization, HTTP, and async runtime crates, along with the rationale for each.  

**DQ‑27:** Describe the exact mechanism by which `js/app/github-api.js` converts a GitHub repository tree API response into a `cc‑vfs` snapshot JSON string, including how file contents are fetched and base64‑decoded.  

**DQ‑28:** Provide a detailed flowchart (textual) for the `WiringManifest` instantiation process in `core/engine/src/wiring.rs`, showing the creation of each major component and the passing of shared references.  

**DQ‑29:** Detail the exact algorithm used by the `PathCanonicalizer` to resolve `../` segments against the `cc‑vfs` root without accessing the host filesystem, including how attempts to escape the root are detected and blocked.  

**DQ‑30:** Provide a complete list of all file paths that are considered “entry files” for the purpose of `CC‑ZERO` banned‑token checks, and explain why each is included.  

**DQ‑31:** Describe the exact process by which the `cc‑token‑walker` builds a `ModuleGraph` from a set of source files, including how it handles nested imports and re‑exports.  

**DQ‑32:** Provide a full specification for the `connectors/chatgpt/manifest.aln` file, including all required fields (`name`, `version`, `entrypoint`, `capabilities`, `policy_overrides`) and an example.  

**DQ‑33:** Detail the exact sequence of `postMessage` calls used to communicate between a Web Worker running the WASM module and the main thread hosting the UI, if this architecture is adopted.  

**DQ‑34:** Provide a complete example of a `ValidationResult` entry for a `CC‑SOV` violation where `use serde_json` is detected in a Rust file, including all fields (`tag`, `passed`, `message`, `path`, `line`, `column`, `severity`).  

**DQ‑35:** Describe the exact algorithm for merging two `cc‑vfs` snapshots, handling conflicts where the same file path exists in both snapshots with different content or SHA values.  

**DQ‑36:** Provide a step‑by‑step description of how the `navigator.rs` `walkdir` function caches directory listings in the `.cc‑cache` directory and invalidates the cache when a `writefile` or `deletefile` task modifies the `cc‑vfs`.  

**DQ‑37:** Detail the exact format of the “professional report” emitted for a `(*/)` detection, including a sample report with all required sections filled in.  

**DQ‑38:** Provide a complete list of the JavaScript language features that the custom interpreter must support, along with the exact syntax for each (e.g., `var x = 10;`, `function foo() { return 42; }`, `if (x > 5) { ... }`).  

**DQ‑39:** Describe the exact mechanism by which the `TaskQueue` enforces the “no write on validation fail” rule, including how it rolls back any partial `cc‑vfs` changes if a later task in the queue fails validation.  

**DQ‑40:** Provide a detailed specification for the `cc‑bench‑token‑walker` WASM export, including its input (a JSON string containing `code` and `profile`) and output (a JSON string with `time_ms`, `throughput_line`, `throughput_byte`, and `symbol_count`).  

**DQ‑41:** Detail the exact sequence of operations that occur when the user selects a file in the file‑tree component: how the path is sent to the Rust core, how the file content is retrieved from the `cc‑vfs`, and how the editor is updated.  

**DQ‑42:** Provide a complete listing of all `extern "C"` functions exported from `cpp/fallback/path-sanitizer.cpp` that are callable from Rust, including their signatures and safety preconditions.  

**DQ‑43:** Describe the exact algorithm for diffing two `CCTokenWalker` symbol sets to produce the set of new declarations introduced in a conversation delta, and how this diff is serialized for `CC‑CRATE` validation.  

---

## Objection Identifiers – Blacklisting & Wiring  

**OI‑11:** **Objection: The requirement to scan every file for `(*/)` on every operation adds unacceptable overhead and may slow down validation in large repositories.**  
*Counter‑strategy:* The `Blacklist` scan is a simple substring search that runs in `O(N)` time with a very low constant factor; it can be enabled only for files that are modified in the current delta, not the entire repository.  

**OI‑12:** **Objection: Building a full import graph across Rust, JavaScript, and C++ using only a custom token walker is too complex and error‑prone.**  
*Counter‑strategy:* The token walker uses language‑specific keyword tables and a state machine that has been proven correct for the subset of syntax needed (e.g., `use`, `require`, `#include`); edge cases are handled by a conservative whitelist approach.  

**OI‑13:** **Objection: The `.cc‑cache` directory violates `CC‑ZERO` because it requires write access to the repository root, which may not be available in some AI‑Chat environments.**  
*Counter‑strategy:* The cache is optional; if write access is unavailable, the agent operates in memory‑only mode and skips caching. The `CC‑ZERO` tag allows repository‑local writes as long as they do not require external setup.  

**OI‑14:** **Objection: Wiring together `Validator`, `Vfs`, `Navigator`, `TaskQueue`, and `TokenWalker` with explicit ownership transfer is overly rigid and makes testing difficult.**  
*Counter‑strategy:* The `WiringManifest` pattern allows dependency injection at startup; tests can supply mock implementations of each trait without modifying the production wiring code.  

**OI‑15:** **Objection: The “connector” abstraction is underspecified and will lead to platform‑specific hacks that break `CC‑SOV`.**  
*Counter‑strategy:* Connectors are confined to the `connectors/` directory, which is subject to the same `CC‑` tag enforcement as core code; each connector must pass all validation before being accepted into the repository.  

**OI‑16:** **Objection: The custom JSON builder cannot handle all valid JSON edge cases (e.g., Unicode escapes, control characters) and will produce invalid output.**  
*Counter‑strategy:* The builder only needs to produce a restricted subset of JSON (no Unicode escapes, no control characters) that is sufficient for `cc‑vfs` snapshots and `ValidationResult` objects; this subset is fully covered by the implementation.  

**OI‑17:** **Objection: The `PathCanonicalizer` that operates purely on the `cc‑vfs` snapshot cannot resolve symbolic links correctly because the snapshot does not contain link information.**  
*Counter‑strategy:* Symbolic links are not supported in the `cc‑vfs` snapshot format; any repository containing symlinks is rejected during snapshot creation with a `CC‑PATH` violation.  

**OI‑18:** **Objection: The `DeepWalker` iterator that enforces depth ≥ 3 for core modules will reject legitimate utility files that happen to live at shallower depths.**  
*Counter‑strategy:* The `CC‑DEEP` check distinguishes between “core” and “leaf” files based on path patterns defined in `invariants.aln`; leaf utilities are explicitly exempted.  

**OI‑19:** **Objection: Maintaining a whitelist of permitted imports for `CC‑SOV` requires constant updates as the repository grows, creating a maintenance burden.**  
*Counter‑strategy:* The whitelist is generated dynamically from the repository’s own file tree; any file with a valid `CC‑FILE` header is automatically allowed, eliminating manual whitelist maintenance.  

**OI‑20:** **Objection: The custom JavaScript interpreter cannot possibly support enough of the language to run realistic connector adapters.**  
*Counter‑strategy:* Connector adapters are written in a minimal subset of JavaScript that avoids dynamic features; the interpreter only needs to support this subset, which is fully specified and implemented.  

---

*End of Research Agenda Supplement*  
*This document contains 100 items (RQ‑35 to OI‑20) as requested.*
