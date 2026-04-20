# Code-Command Blacklisting & Wiring Research Agenda  
**Version:** 1.0.0  
**Status:** Active Inquiry  
**Purpose:** This agenda provides an additional 100 research questions, definition requests, detail queries, and objection identifiers specifically focused on **blacklisting capabilities** (detection, reporting, policy enforcement) and **wiring together components** (module interconnection, dependency injection, cross‑language FFI, event routing) within the Code‑Command repository. It extends the prior research agendas with precise definitions for directories, filenames, and inter‑component communication paths.

---

## Table of Contents  
1. [Research Questions – Blacklisting & Wiring](#research-questions-blacklisting--wiring) – 40 items  
2. [Definition Requests – Blacklisting & Wiring](#definition-requests-blacklisting--wiring) – 30 items  
3. [Detail Queries – Blacklisting & Wiring](#detail-queries-blacklisting--wiring) – 20 items  
4. [Objection Identifiers – Blacklisting & Wiring](#objection-identifiers-blacklisting--wiring) – 10 items  

---

## Research Questions – Blacklisting & Wiring  

**RQ‑201:** How can the blacklist scanner be extended to support pattern‑matching on tokenized input (e.g., matching `serde::Serialize` as a banned type annotation) without introducing a full parser and while maintaining `O(N)` time and zero allocations?  

**RQ‑202:** What is the most efficient in‑memory data structure for storing and querying a dynamic blacklist rule set that includes literal strings, simple globs, and language‑specific contexts, allowing `O(1)` amortized checks per token?  

**RQ‑203:** How can the `BlacklistRule` parser in `cc‑policy` be designed to load rules from multiple `.aln` files with a well‑defined precedence order (repository root, per‑directory overrides, user‑supplied configuration) while respecting CC‑PATH invariants?  

**RQ‑204:** What algorithm can be used to generate a minimal deterministic finite automaton (DFA) from a set of blacklist string patterns, and how can this DFA be compiled into Rust code at build time to avoid runtime construction overhead?  

**RQ‑205:** How can the blacklist detector be integrated with the `CCTokenWalker` such that banned tokens are recognized in their syntactic context (e.g., as a crate name in a `use` statement versus a string literal), while still avoiding external parser crates?  

**RQ‑206:** What is the exact wiring sequence required to propagate a `ContaminationReport` from the Rust validator through the WASM boundary, into the JavaScript `OutputPanel`, and finally to a professional report format suitable for AI‑Chat platforms?  

**RQ‑207:** How can the `WiringManifest` be extended to support lazy initialization of components (e.g., `Navigator` only instantiated when a `walkdir` is requested) without introducing global mutable state or violating Rust’s ownership rules?  

**RQ‑208:** What is the optimal event‑routing architecture for the JavaScript frontend that allows components (`FileTree`, `Editor`, `OutputPanel`) to subscribe to engine events (`vfs:updated`, `validation:complete`, `task:executed`) using only vanilla DOM events or a minimal custom event bus?  

**RQ‑209:** How can the Rust core expose a structured logging channel to JavaScript that carries log level, component tag, and correlation ID, and how should this channel be wired into the `OutputPanel` for real‑time display?  

**RQ‑210:** What is the exact mechanism for dynamically loading a connector adapter JavaScript file from `connectors/[platform]/adapter.js` in the browser without using `eval` or `import()` with dynamic specifiers, and how does this satisfy CC‑ZERO constraints?  

**RQ‑211:** How can the `TaskQueue` be wired to a pluggable persistence backend such that tasks marked with `profile: "github"` invoke the GitHub bridge while tasks with `profile: "memory‑only"` bypass it, all without changing the core SITQ logic?  

**RQ‑212:** What is the most efficient way to serialize a `WiringManifest` graph (nodes and edges) into a format that can be validated against `specs/wiring‑spec.aln` using a custom, sovereign validator?  

**RQ‑213:** How can the `PathCanonicalizer` be wired to use a chain‑of‑responsibility pattern: first try pure Rust normalization, then fall back to the C++ `cc_sanitize_path` via FFI, and finally reject the path if both fail?  

**RQ‑214:** What is the precise algorithm for generating a collision‑resistant cache key for `.cc‑cache` entries that incorporates the repository owner, repository name, branch/commit SHA, and execution profile, and how is this key used in the JavaScript `Cache` module?  

**RQ‑215:** How can the `Blacklist` detector be wired to run as a pre‑commit hook in the GitHub Action workflow, blocking any PR that introduces a banned token, and how should the failure report be formatted for CI consumption?  

**RQ‑216:** What is the minimal set of `extern "C"` functions required to expose the `CCTokenWalker` functionality to a C or C++ testing harness, and how can these functions be used to verify cross‑language equivalence of symbol collection?  

**RQ‑217:** How can the `Validator` be wired to accept a `PolicyOverride` object that temporarily disables certain tags (e.g., for dry‑run previews) without allowing the override to persist or leak into subsequent validations?  

**RQ‑218:** What is the optimal strategy for wiring the `Navigator` to the `Vfs` in WASM builds so that directory listings are retrieved from the in‑memory snapshot without blocking the main thread?  

**RQ‑219:** How can the `cc‑bench‑token‑walker` export be extended to measure blacklist scanning performance separately, and how should the benchmark results be integrated into the `ResearchObject` metrics?  

**RQ‑220:** What is the exact sequence of `postMessage` calls needed to implement a bidirectional RPC channel between a Web Worker running the WASM engine and the main thread, such that the worker can request file contents from the main thread’s `Vfs` cache?  

**RQ‑221:** How can the `BlacklistRule` set be versioned and signed within `specs/blacklist.aln` to prevent tampering, and how should the validator verify the signature using only sovereign cryptographic primitives (e.g., a custom SHA‑256 implementation)?  

**RQ‑222:** What is the most compact wire format for a `ContaminationReport` that can be embedded in a `ValidationResult` without bloating the JSON payload, and how should this format be documented in `engine.aln`?  

**RQ‑223:** How can the `WiringManifest` be instrumented to collect cross‑module call counts for the “wiring efficiency” metric defined in DR‑55, and how should these metrics be exposed to the JavaScript UI?  

**RQ‑224:** What algorithm can be used to detect cyclic dependencies in the wiring graph (e.g., `Validator` depending on `Vfs` while `Vfs` indirectly calls `Validator`) and report them under a `CC‑WIRING‑CYCLE` tag?  

**RQ‑225:** How can the `Blacklist` detector be configured via ALN to treat certain tokens as “warn” rather than “block”, and how should warnings be rendered in the `ValidationResult` and professional reports?  

**RQ‑226:** What is the exact directory layout for per‑module policy overrides that affect blacklisting, e.g., `core/engine/src/.ccpolicy.aln` tightening blacklist rules for engine code only, and how should the policy merger handle overlapping rules?  

**RQ‑227:** How can the `TaskQueue` be wired to a transactional VFS that supports nested transactions (e.g., a SITQ within a SITQ), and under what circumstances should nested transactions be permitted or rejected?  

**RQ‑228:** What is the most efficient way to serialize a `ModuleGraph` (nodes and edges) into a format that can be visualized in the browser using a simple canvas‑based renderer without external graphing libraries?  

**RQ‑229:** How can the `CCTokenWalker` be extended to recognize and blacklist specific function calls (e.g., `std::env::var`) in entry files for CC‑ZERO, and how should this be integrated with the existing banned‑token list?  

**RQ‑230:** What is the precise mechanism for wiring the `OutputPanel` to receive structured validation failures and display them grouped by file path and tag, with collapsible sections and links to open the offending file?  

**RQ‑231:** How can the `Blacklist` detector be made aware of code comments and string literals to avoid false positives (e.g., `"Tree‑Sitter"` in a doc comment), and what is the minimal state machine required to track comment/string boundaries across Rust, JS, and C++?  

**RQ‑232:** What is the optimal strategy for caching `Blacklist` scan results per file based on content hash, and how should this cache be invalidated when the blacklist rule set changes?  

**RQ‑233:** How can the `WiringManifest` be made serializable to a JSON description that can be used by external tooling (e.g., a diagram generator) to visualize the engine’s internal connections?  

**RQ‑234:** What is the exact sequence of function calls that occurs when a user triggers a “validate all” action in the UI, leading to a SITQ `validateonly` task that scans multiple files and returns aggregated results?  

**RQ‑235:** How can the `PathCanonicalizer` be wired to reject paths that contain blacklisted directory names (e.g., `node_modules`, `target`) as a fast pre‑filter before deeper validation?  

**RQ‑236:** What is the most efficient way to diff two `Blacklist` rule sets (e.g., between policy versions) and report which rules have been added, removed, or modified?  

**RQ‑237:** How can the JavaScript `github‑api.js` module be wired to respect the `Blacklist` policy by refusing to fetch or display files that are known to be blacklisted (e.g., files containing banned tokens)?  

**RQ‑238:** What is the exact format for a “professional report” that summarizes multiple blacklist violations across several files, and how should this report be presented to an AI‑Chat platform for user remediation?  

**RQ‑239:** How can the `cc‑policy` parser be extended to support conditional blacklist rules based on the active profile (e.g., stricter rules for `github` profile than for `memory‑only`)?  

**RQ‑240:** What is the most efficient algorithm for merging multiple `Blacklist` policy files with inheritance and override semantics, resolving conflicts using a deterministic “most specific wins” rule?  

---

## Definition Requests – Blacklisting & Wiring  

**DR‑64:** Define the exact ALN syntax for a `BlacklistRule` that includes a `pattern`, `pattern_type` (`literal`, `glob`, `regex‑subset`), `languages` array, `context` (`import`, `declaration`, `any`), `severity` (`block`, `warn`), and `message` template.  

**DR‑65:** Define the term “blacklist context” as used in the token walker: the syntactic environment in which a token appears (e.g., inside an import statement, inside a string literal, inside a comment) and how the walker determines this context without a full AST.  

**DR‑66:** Define the exact shape of a `WiringEdge` in the wiring graph, including fields `from_module`, `to_module`, `call_site` (function name), `line`, and `is_optional`.  

**DR‑67:** Define the internal representation of a `BlacklistMatch` that is produced when a rule triggers, including fields `rule_id`, `pattern`, `matched_text`, `location` (`path`, `line`, `column`), `context`, and `severity`.  

**DR‑68:** Define the term “wiring contract” as a machine‑readable specification in `specs/wiring‑spec.aln` that declares which module calls are allowed and which are forbidden, and how this contract is enforced at build time or runtime.  

**DR‑69:** Define the exact filename and location for per‑directory blacklist overrides, e.g., `.ccblacklist.aln`, and specify how these files are discovered and merged with the global blacklist.  

**DR‑70:** Define the internal data structure for a `BlacklistProfile` that combines a set of active rules, a language filter, and a severity threshold, and how this profile is constructed from policy files.  

**DR‑71:** Define the term “event bus” in the context of the JavaScript frontend: a minimal publish‑subscribe system with methods `on(event, callback)`, `off(event, callback)`, and `emit(event, payload)`, and its required thread‑safety guarantees.  

**DR‑72:** Define the exact JSON schema for a `WiringManifest` serialization, including fields `nodes` (array of `{ id, type }`) and `edges` (array of `{ from, to, via }`).  

**DR‑73:** Define the term “blacklist incident” as a formal event that triggers a professional report, and list the required metadata: `incident_id`, `timestamp`, `detector_version`, `rule_id`, `location`, `excerpt`, and `recommended_action`.  

**DR‑74:** Define the exact API of the `EventRouter` Rust trait that allows components to subscribe to engine events, including methods `subscribe(&mut self, event: EventType, callback: Box<dyn Fn(Event)>)` and `emit(&self, event: Event)`.  

**DR‑75:** Define the internal representation of a `PolicyOverride` struct that can temporarily modify tag activation and blacklist rules for a single validation request, and how it is applied without mutating global state.  

**DR‑76:** Define the term “connector sandbox” as the set of allowed JavaScript APIs and CC‑API functions that a connector adapter may call, and list the exact banned features (e.g., `eval`, `Function`, `import()`, `fetch` to non‑GitHub domains).  

**DR‑77:** Define the exact grammar for a `Blacklist` entry that uses a simple glob syntax (e.g., `serde::*`) and how glob expansion is performed during scanning without external crates.  

**DR‑78:** Define the internal representation of a `ComponentRegistry` that holds references to all major engine components (`Vfs`, `Validator`, `Navigator`, `TaskQueue`) and allows them to be looked up by type for dependency injection.  

**DR‑79:** Define the term “wiring efficiency” as a measurable metric: the number of cross‑module function calls required to process a single `validateonly` task, and how this metric is computed from the `WiringManifest` graph.  

**DR‑80:** Define the exact format of a “blacklist summary report” that aggregates multiple `BlacklistMatch` entries across a repository, including total counts, rule breakdowns, and a severity histogram.  

**DR‑81:** Define the internal data structure for a `LazyComponent<T>` that wraps an optional `T` and initializes it on first access using a provided closure, and how this is used in `WiringManifest` for lazy instantiation.  

**DR‑82:** Define the term “cross‑cutting concern” in the wiring graph, such as logging or metrics collection, and describe how such concerns are injected into the call graph without modifying core business logic.  

**DR‑83:** Define the exact ALN syntax for declaring a `WiringManifest` in `specs/wiring‑spec.aln`, including sections for `nodes`, `edges`, and `invariants` (e.g., “`TaskQueue` must not call `Navigator` directly”).  

**DR‑84:** Define the internal representation of a `BlacklistCache` that maps file content hashes to scan results, including fields `hash`, `timestamp`, `matches` array, and `ttl`.  

**DR‑85:** Define the term “dependency injection container” in the context of Code‑Command’s Rust core, and how it differs from traditional DI frameworks by being compile‑time and zero‑cost.  

**DR‑86:** Define the exact filename and location for a connector’s sandbox policy file, e.g., `connectors/[platform]/sandbox.aln`, and the required fields (`allowed_apis`, `banned_globals`, `max_runtime_ms`).  

**DR‑87:** Define the internal data structure for a `BlacklistRuleSet` that organizes rules by language and context for fast lookup during scanning, e.g., a `HashMap<Language, Vec<Rule>>`.  

**DR‑88:** Define the term “event correlation ID” as a unique identifier that ties together all log entries, validation results, and task reports for a single user action, and how this ID is propagated across the Rust‑JS boundary.  

**DR‑89:** Define the exact API of the `WiringValidator` that checks a `WiringManifest` against `specs/wiring‑spec.aln` and returns a `ValidationResult` with any violations.  

**DR‑90:** Define the internal representation of a `BlacklistExemption` that allows a specific file or directory to bypass certain blacklist rules, and how exemptions are declared in `.ccpolicy.aln`.  

**DR‑91:** Define the term “pluggable validator” as a component that can be registered with the `Validator` to handle custom tags, and describe the trait bounds required for such plug‑ins.  

**DR‑92:** Define the exact byte‑level format of a `BlacklistPattern` that uses a simple regular expression subset (character classes, `*`, `+`, `?`, `|`), and how this subset is compiled into a state machine without external regex crates.  

**DR‑93:** Define the term “wiring graph validation” as the process of ensuring that the actual call graph at runtime matches the expected graph declared in `wiring‑spec.aln`, and how this is enforced via static analysis or runtime checks.  

---

## Detail Queries – Blacklisting & Wiring  

**DQ‑44:** Provide a step‑by‑step description of how the `Blacklist` detector is initialized from `specs/blacklist.aln` and `.ccblacklist.aln` files during engine startup, including the parsing, merging, and compilation into a `BlacklistProfile`.  

**DQ‑45:** Detail the exact sequence of function calls that occurs when a `ContaminationReport` is generated, from the moment a banned token is detected in `validator.rs` to the point where the report is appended to the `ValidationResult` JSON.  

**DQ‑46:** Provide a complete listing of all event types that the JavaScript event bus must support (`vfs:mounted`, `vfs:updated`, `file:opened`, `file:saved`, `validation:started`, `validation:completed`, `task:queued`, `task:completed`, `task:failed`), along with the payload shape for each.  

**DQ‑47:** Describe the exact mechanism by which the `WiringManifest` is instantiated in `lib.rs`, including the order of component creation, the passing of shared references, and the handling of any initialization failures.  

**DQ‑48:** Provide a detailed flowchart (textual) for the `Blacklist` scanning process within the token walker, showing how the scanner moves through the code bytes, tracks context (comment, string, import), and checks against the compiled rule set.  

**DQ‑49:** Detail the exact format of a “professional report” for a single blacklist violation, including a sample report with all fields populated, as it would be returned to an AI‑Chat platform.  

**DQ‑50:** Provide a complete list of all `extern "C"` functions that must be exported from a C implementation of the `CCTokenWalker` to allow the Rust engine to call it for cross‑language validation, along with their signatures and safety preconditions.  

**DQ‑51:** Describe the exact process by which the `FileTree` component subscribes to `vfs:updated` events and refreshes its DOM representation, including the diffing algorithm used to minimize re‑renders.  

**DQ‑52:** Provide a step‑by‑step description of how the `cc‑policy` parser extracts `Blacklist` rules from an ALN file, including the handling of YAML frontmatter and the mapping of ALN fields to `BlacklistRule` struct fields.  

**DQ‑53:** Detail the exact sequence of `postMessage` calls used to implement a request‑response pattern between the main thread and a Web Worker running the WASM engine, including the generation and matching of correlation IDs.  

**DQ‑54:** Provide a complete example of a `WiringManifest` JSON serialization for the current engine, including all nodes (`Lib`, `Validator`, `Vfs`, `Navigator`, `TaskQueue`, `TokenWalker`) and the expected edges between them.  

**DQ‑55:** Describe the exact algorithm used by the `BlacklistCache` to determine whether a cached scan result is still valid, based on file content hash and the version of the blacklist rule set.  

**DQ‑56:** Provide a detailed specification for a new WASM export `cc_scan_blacklist(code: string, profile_json: string) -> string` that returns a JSON array of `BlacklistMatch` objects without running full validation.  

**DQ‑57:** Detail the exact mechanism by which the `OutputPanel` renders a `ContaminationReport` in a distinct visual style (e.g., red background, warning icon) to differentiate it from ordinary validation failures.  

**DQ‑58:** Provide a complete listing of all banned global identifiers that the connector sandbox must reject in JavaScript adapter files, including `window`, `document`, `localStorage`, `fetch` (except to GitHub API), `WebSocket`, and `eval`.  

**DQ‑59:** Describe the exact process by which the `TaskQueue` selects the appropriate persistence backend based on the task’s `profile` field, and how the backend is invoked to commit changes after successful validation.  

**DQ‑60:** Provide a step‑by‑step description of how the `WiringValidator` checks a `WiringManifest` against `wiring‑spec.aln`, including how missing edges and forbidden edges are reported.  

**DQ‑61:** Detail the exact format of a “blacklist summary report” JSON object that aggregates violations across a SITQ run, including fields `total_violations`, `by_rule`, `by_file`, and `severity_counts`.  

**DQ‑62:** Provide a complete example of a `.ccblacklist.aln` file that overrides the global blacklist for a specific subdirectory, including the frontmatter and a list of additional rules.  

**DQ‑63:** Describe the exact sequence of operations that occur when the user clicks a “Fix CC‑SOV” button in the UI, leading to a SITQ task that rewrites the offending import to use a sovereign alternative, and how the editor is updated afterward.  

---

## Objection Identifiers – Blacklisting & Wiring  

**OI‑21:** **Objection: Maintaining a context‑aware blacklist scanner (e.g., distinguishing import statements from string literals) requires a full parser and violates the “no external parser” rule.**  
*Counter‑strategy:* A simple state machine tracking comment and string boundaries is sufficient for the required accuracy; false positives in edge cases are acceptable because the user can request an exemption or rewrite the code to avoid the banned token.  

**OI‑22:** **Objection: The proposed event bus for the JavaScript frontend adds unnecessary complexity when simple function calls would suffice.**  
*Counter‑strategy:* The event bus decouples UI components, allowing them to be developed and tested independently; it is implemented as a minimal 50‑line module with no external dependencies.  

**OI‑23:** **Objection: Wiring validation against `wiring‑spec.aln` is over‑engineering and will break frequently as the codebase evolves.**  
*Counter‑strategy:* The wiring spec is optional for development builds but enforced in CI; it serves as living documentation and a safeguard against accidental architectural violations.  

**OI‑24:** **Objection: Caching blacklist scan results per file may lead to stale results if the blacklist rule set is updated but the cache is not invalidated.**  
*Counter‑strategy:* The cache key includes a hash of the active rule set; any change to the rules automatically invalidates all cached entries.  

**OI‑25:** **Objection: The connector sandbox restrictions are too severe and will prevent useful connectors from being written.**  
*Counter‑strategy:* Connectors that require additional capabilities can request them via explicit declaration in `sandbox.aln`; the engine can then prompt the user for permission or reject the connector if the capabilities are not allowed.  

**OI‑26:** **Objection: Serializing the `WiringManifest` to JSON for validation is wasteful and the same checks could be done statically with a lint tool.**  
*Counter‑strategy:* The serialization is only performed in debug builds or during conformance testing; it provides a machine‑readable snapshot that can be compared against the spec, ensuring that the runtime wiring matches the documented architecture.  

**OI‑27:** **Objection: The `Blacklist` detector’s context tracking (comments, strings) will be fragile across different language syntaxes (e.g., Rust raw strings, JS template literals).**  
*Counter‑strategy:* The context tracker uses a conservative approach: when in doubt, it treats the region as “code” rather than “comment/string,” which may cause false positives but never false negatives, preserving safety.  

**OI‑28:** **Objection: The `EventRouter` trait in Rust requires dynamic dispatch (`Box<dyn Fn>`) which introduces allocation and indirection overhead.**  
*Counter‑strategy:* The `EventRouter` is used only for infrequent events (e.g., VFS mount, task completion); the hot path (validation) does not use it, so the performance impact is negligible.  

**OI‑29:** **Objection: Generating professional reports for blacklist violations is redundant; a simple error message would suffice.**  
*Counter‑strategy:* Professional reports are required by the (R) rules to provide a full accounting of contamination incidents; they ensure that users and AI‑Chat platforms understand the severity and required remediation steps.  

**OI‑30:** **Objection: The `WiringManifest` and component registry introduce “magic” global state that makes the engine harder to reason about.**  
*Counter‑strategy:* The manifest is constructed once at startup and then treated as immutable; all components receive explicit references to their dependencies, preserving clear ownership and lifetimes.  

---

*End of Blacklisting & Wiring Research Agenda*  
*This document contains 100 items (RQ‑201 to OI‑30) as requested.*
