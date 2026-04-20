# Code-Command 100 Coding Tasks  
**Derived from:** Code‑Command Blacklisting & Wiring Research Agenda (RQ‑201–OI‑30)  
**Purpose:** Concrete, implementable work items to advance the Code‑Command engine, policy layer, WASM exports, JavaScript UI, connectors, and testing infrastructure.  

---

## Tasks 1–100

1. **Rust/Core — Implement BlacklistCache with hash and rules_version keys**  
   Create `core/engine/src/blacklist_cache.rs` containing `BlacklistCacheEntry` struct with fields `hash: String`, `rules_version: u64`, `timestamp: u64`, `matches: Vec<BlacklistMatch>`, and `ttl: u64`. Implement `BlacklistCache` as a `HashMap<String, BlacklistCacheEntry>` keyed by normalized VFS path. Provide `is_valid(path, current_hash, current_rules_version, now)` method that returns true only when the entry exists, hashes match, versions match, and TTL has not expired. Integrate cache lookups into `Validator::run_validation` to avoid rescanning unchanged files when blacklist rules are unchanged.

2. **Rust/Core — Add BlacklistProfile struct with rules and language filters**  
   In `core/engine/src/blacklist.rs`, define `BlacklistProfile` containing `profile_name: String`, `rules: Vec<BlacklistRule>`, `language: LanguageHint`, and `min_severity: BlacklistSeverity`. Provide a builder `from_policy(policy: &Policy, language: LanguageHint, profile_name: &str)` that merges global and per‑directory blacklist rules, applies profile‑specific overrides, and filters rules by language compatibility. Use this profile during blacklist scanning instead of ad‑hoc rule collection.

3. **Rust/Core — Implement BlacklistRuleSet with language/context buckets for fast lookup**  
   Extend `core/engine/src/blacklist.rs` with `BlacklistRuleSet` struct containing a `HashMap<(RuleLanguage, RuleContext), Vec<BlacklistRule>>`. Implement `from_rules(rules: Vec<BlacklistRule>)` that populates buckets by expanding each rule’s `languages` list and `context` field. Provide `candidate_rules(language: RuleLanguage, context: RuleContext)` that returns an iterator over matching rules (including fallback to `Any` language/context). Use this in `CCTokenWalker` to quickly select rules for a given scanning context.

4. **Rust/Core — Implement byte‑level blacklist pattern DFA compiler and matcher**  
   Create `core/engine/src/blacklist_pattern.rs` with `BlacklistPattern` struct containing `raw: String`, `is_regex: bool`, and `ops: Vec<PatternOp>`. Implement a Thompson NFA compiler for a restricted regex subset: literal bytes, `.` (any char), character classes `[...]`, quantifiers `*`, `+`, `?`, and alternation `|`. Provide `compile(pattern: &str, is_regex: bool) -> BlacklistPattern` and `matches(&self, haystack: &[u8]) -> Option<(usize, usize)>`. Integrate into `CCTokenWalker::scan_blacklist_segment` to replace substring search for rules with `pattern_type: "regex-subset"`.

5. **Rust/Core — Add context‑tracking state machine to CCTokenWalker**  
   Modify `CCTokenWalker` in `core/engine/src/token_walker.rs` to maintain a lexical state: `Code`, `LineComment`, `BlockComment`, `StringSingle`, `StringDouble`, `StringBacktick` (JS only). Transitions triggered by `//`, `/*`, `*/`, quotes, and backslashes for escapes. During scanning, only invoke blacklist checks when state is `Code` (or `Import`/`Declaration` based on line prefix). Ensure the state machine correctly skips comments and strings for language‑specific rules while allowing hard markers to scan all states.

6. **Rust/Core — Implement BlacklistExemption struct and integration in Validator**  
   Define `BlacklistExemption` in `core/engine/src/blacklist.rs` with fields `path_prefix: String` and `rule_ids: Vec<String>`. Extend `BlacklistProfile` to hold `exemptions: Vec<BlacklistExemption>`. During scanning, filter out any `BlacklistRule` whose `id` appears in an exemption whose `path_prefix` matches the current file’s path prefix. Ensure exemptions are loaded from `.ccpolicy.aln` files using `blacklist_exemptions:` ALN sections.

7. **Rust/Core — Implement PluggableValidator trait and registry**  
   Create `core/engine/src/validator_plugin.rs` with `PluggableValidator` trait defining `supported_tags(&self) -> &'static [&'static str]` and `validate(&self, code: &str, path: &str, language: LanguageHint) -> Vec<ValidationEntry>`. Modify `Validator` to hold `plugins: Vec<Box<dyn PluggableValidator>>` and call `run_plugins` for any requested tag matching a plugin’s supported tags. This allows custom tags to be added without modifying core validator.

8. **Rust/Core — Implement WiringManifest struct and connect_all factory**  
   Create `core/engine/src/wiring.rs` with `WiringManifest` holding owned instances of `Vfs`, `Validator`, `Navigator`, `TaskQueue`, and `TokenWalker`. Implement `connect_all() -> Result<WiringManifest, WiringError>` that constructs components in order: `Vfs::new()`, `TokenWalker::new()`, `Policy::load_specs(&vfs)`, `Validator::with_token_walker(...)`, `Navigator::with_vfs_handle(...)`, `TaskQueue::new()`. Store the manifest in a static `OnceCell` accessible to WASM exports.

9. **Rust/Core — Implement LazyComponent<T> for on‑demand component instantiation**  
   In `core/engine/src/wiring.rs`, add `LazyComponent<T, F>` with `new(init_fn: F)`, `get_mut(&mut self) -> &mut T`, and `get(&self) -> &T` (using interior mutability). Use this to wrap `Navigator` and `TaskQueue` in `WiringManifest` so they are constructed only when first accessed, reducing startup cost for profiles that do not use them.

10. **Rust/Core — Add WiringGraph serialization to WiringManifest**  
    Implement `WiringManifest::to_graph() -> WiringGraph` that builds nodes for `Lib`, `Validator`, `Vfs`, `Navigator`, `TaskQueue`, `TokenWalker` and edges for known call patterns (`Lib→Vfs`, `Lib→Validator`, `Lib→TaskQueue`, `TaskQueue→Vfs`, `TaskQueue→Validator`, `Validator→TokenWalker`, `Navigator→Vfs`). Provide `to_json() -> String` using `JsonBuilder` with short keys (`n` for nodes, `e` for edges). This JSON is used by `WiringValidator` and external visualization tools.

11. **Rust/Core — Implement WiringValidator to check WiringManifest against wiring‑spec.aln**  
    Create `core/engine/src/wiring_validator.rs` with `WiringValidator` struct that loads `specs/wiring‑spec.aln` via `Vfs::read` and parses `nodes`, `required_edges`, and `forbidden_edges`. Implement `validate(actual: &WiringGraph) -> ValidationResult` that checks: all expected nodes exist; each required edge has at least one matching `via`; no forbidden edge exists. Return failures tagged `CC‑WIRING`.

12. **Rust/Core — Extend TaskQueue to support nested transactions with TransactionalVfs**  
    Modify `TaskQueue` to use a new `TransactionalVfs` wrapper that maintains a stack of overlays. Implement `begin_tx()`, `commit_tx()`, `rollback_tx()` on `Vfs`. Change `TaskQueue::execute` to call `vfs.begin_tx()` at start and either `commit_tx()` on success or `rollback_tx()` on failure. Allow nested SITQ calls (e.g., a composite task) to call `begin_tx()` again, with inner transaction effects contained until committed.

13. **Rust/Core — Add profile‑based persistence backend selection in TaskQueue**  
    In `TaskQueue::execute`, read `profile` field from `TaskQueuePayload`. Map `"github"` to `PersistenceBackend::Github`, `"local"` to `Local`, `"memory‑only"` to `MemoryOnly`. After successful execution, if profile is not `MemoryOnly`, include a `persist_changes` array in `TaskReport` with file operations. The Rust core does not perform network IO; the JS host uses this array to call GitHub/local APIs. Unknown profiles cause error `CCENG‑030`.

14. **Rust/Core — Implement PathCanonicalizer chain of responsibility with C++ fallback**  
    In `core/engine/src/path.rs`, create `PathCanonicalizer` with `canonicalize(raw: &str) -> Option<String>`. First run pure Rust normalization (replace backslashes, collapse slashes, resolve `.` and `..` using stack). If Rust fails and `cfg(feature = "cpp-fallback")` is set, call `cc_sanitize_path` via FFI. Re‑validate the result with Rust’s `is_valid_path`. Use this canonicalizer in `Vfs::write`, `Navigator::walkdir`, and `TaskQueue` to enforce consistent path handling.

15. **Rust/Core — Implement deterministic cache key generation for .cc‑cache**  
    In `core/engine/src/cache_key.rs`, provide `fn cache_key(owner: &str, repo: &str, ref_name: &str, profile: &str) -> String`. Concatenate fields with `/` separators, compute a custom FNV‑1a 64‑bit hash, and format as `{owner}-{repo}-{profile}-{hash:x}`. Use this key to namespace VFS paths under `.cc‑cache/`. Expose via `cc_compute_cache_key` WASM export for JS to use in IndexedDB namespacing.

16. **Rust/Core — Add cc_scan_blacklist WASM export**  
    In `lib.rs`, implement `#[wasm_bindgen] pub fn cc_scan_blacklist(code: &str, profile_json: &str) -> String`. Parse `profile_json` into `BlacklistScanProfile` with `language` and `tags`. Build `ScanProfile` with `BIT_BLACKLIST` enabled. Retrieve `BlacklistProfile` from engine, run `TokenWalker::scan_blacklist_only`, and return JSON array of `BlacklistMatch` objects (as defined in DQ‑56). This export allows pre‑scanning without full validation.

17. **Rust/Core — Add cc_get_wiring_json WASM export**  
    In `lib.rs`, add `#[wasm_bindgen] pub fn cc_get_wiring_json() -> String`. Access the global `WiringManifest`, call `to_graph().to_json()`, and return the compact JSON string. This enables external tools (diagram generators, CI) to inspect the actual engine wiring.

18. **Rust/Core — Add cc_poll_logs WASM export for structured logging**  
    Implement a ring buffer in the engine that collects `LogRecord` structs (`level`, `component`, `message`, `correlation_id`, `timestamp`). Expose `#[wasm_bindgen] pub fn cc_poll_logs() -> String` returning JSON array of pending records and clearing the buffer. Wire major engine events (`TaskQueue` start/finish, `Validator` failures, blacklist hits) to push records. JS will periodically call this export and dispatch to `OutputPanel`.

19. **Rust/Core — Extend cc_bench_token_walker to measure blacklist scanning separately**  
    Modify `cc_bench_token_walker` to accept `want_blacklist: bool` in profile JSON. Run two passes: one with blacklist disabled (baseline), one with blacklist enabled. Return JSON with fields `time_ms_base`, `time_ms_with_blacklist`, `throughput_line`, `throughput_byte`, `symbol_count`, and `blacklist_hits`. Integrate these metrics into `ResearchObject` under `tw.time_ms_base` and `tw.time_ms_blacklist`.

20. **Rust/Core — Add CC‑ZERO call‑pattern blacklisting in token walker**  
    In `CCTokenWalker`, when scanning Rust entry files (`LanguageHint::Rust` and marked as entry), look for function call patterns like `std::env::var` by detecting identifier sequences followed by `(`. Normalize to a string (e.g., `std::env::var`) and check against a `HashSet<String>` of forbidden calls loaded from `invariants.aln` under `cc_zero_forbidden_calls`. On match, emit a `CC‑ZERO` failure with location.

21. **Rust/Core — Implement blacklist rule diffing for policy version comparison**  
    Create `core/engine/src/blacklist_diff.rs` with `diff_rules(old: &[BlacklistRule], new: &[BlacklistRule]) -> BlacklistDiff`. Use `HashMap` keyed by `id`. Compute `added_ids`, `removed_ids`, and `modified` (comparing normalized fields). Return a JSON‑serializable struct. Use this in `cc_policy` to report policy changes and to invalidate caches when rules change.

22. **Rust/Core — Add blacklist summary report generation in TaskQueue**  
    After `TaskQueue::execute` completes, aggregate all `BlacklistMatch` instances collected during validation into a `BlacklistSummary` struct (fields: `total_violations`, `severity_counts`, `by_rule`, `by_file`). Serialize to JSON with `kind: "BLACKLIST‑SUMMARY‑1"` and attach to `TaskReport` as `blacklist_summary`. This provides a compact overview for CI and dashboards.

23. **Rust/Core — Implement EventRouter trait with dynamic dispatch**  
    In `core/engine/src/event_router.rs`, define `EventType` enum (`VfsUpdated`, `ValidationCompleted`, `BlacklistIncident`, `WiringChanged`, `Custom(String)`) and `Event` struct. Define `EventRouter` trait with `subscribe(&mut self, event: EventType, callback: Box<dyn Fn(&Event) + 'static>)` and `emit(&self, event: &Event)`. Provide a simple implementation `SimpleEventRouter` that stores callbacks in a `HashMap`. Use this only for non‑hot‑path events.

24. **Rust/Core — Emit VfsUpdated event after successful SITQ writes**  
    Inside `TaskQueue::execute`, after successfully committing a transaction that modifies VFS, construct an `Event::VfsUpdated` with payload containing `vfs_id` and list of changed paths. Emit via the global `EventRouter` instance. The JS side can listen to this via a dedicated channel to refresh UI.

25. **Rust/Core — Emit BlacklistIncident event when a block‑level blacklist hit occurs**  
    When `CCTokenWalker` detects a blacklist match with `severity: "block"`, immediately construct a `BlacklistIncident` struct with `incident_id`, `timestamp`, `rule_id`, `location`, and `excerpt`. Emit via `EventRouter` as `EventType::BlacklistIncident`. This allows connectors and UI to react in real‑time, e.g., by showing a modal.

26. **Rust/Core — Implement ComponentRegistry for explicit dependency injection**  
    Create `core/engine/src/component_registry.rs` with `ComponentRegistry<'a>` holding `Option<&'a mut Vfs>`, `Option<&'a mut Validator>`, etc. Provide builder methods `with_vfs`, `with_validator`, etc. Use this in `wiring.rs` as an alternative to `WiringManifest` for tests, allowing mock implementations to be injected.

27. **Rust/Core — Add CC‑CYCLE detection using ModuleGraph**  
    Build a `ModuleGraph` from `CCTokenWalker` import scans across the repository. Implement `detect_cycles() -> Vec<Vec<String>>` using DFS with recursion stack or Tarjan’s SCC algorithm. For each cycle, emit a `ValidationEntry` with tag `CC‑CYCLE` and message listing the cyclic modules. Integrate this as an optional check gated by a tag in `invariants.aln`.

28. **Rust/Core — Implement PathCanonicalizer blacklisted directory pre‑filter**  
    Extend `PathCanonicalizer::canonicalize` to check each path segment (except the last if file) against a `HashSet<String>` of blacklisted directory names (e.g., `node_modules`, `target`, `.git`). If any segment matches, return `None` and emit a `CC‑PATH` failure. Load the blacklist from `specs/path‑policy.aln` or `invariants.aln`.

29. **Rust/Core — Add dry‑run mode to TaskQueue via PolicyOverride**  
    In `TaskQueuePayload`, add optional `dry_run: bool`. When true, `TaskQueue::execute` clones the VFS, applies tasks to the clone, validates, but does not commit to the real VFS or invoke persistence backends. Return a `TaskReport` containing a `simulated_snapshot` field with the projected VFS state. This allows previewing changes.

30. **Rust/Core — Implement PolicyOverride struct and apply per‑request**  
    Define `PolicyOverride` with `tags_enable: Vec<String>`, `tags_disable: Vec<String>`, `extra_rules: Vec<BlacklistRule>`, and `rule_patches: Vec<BlacklistRule>`. In `Validator::run_validation`, accept an `Option<PolicyOverride>`. Clone the global `PolicyProfile`, apply tag toggles and rule patches, and use the modified profile for that validation only. Do not mutate global state.

31. **Rust/Core — Add per‑task policy_override support in SITQ**  
    Extend the `Task` JSON schema to include an optional `policy_override` object mirroring `PolicyOverride`. During `TaskQueue::execute`, when processing a task, merge its override with the queue‑level override (if any) and pass to `Validator`. This enables fine‑grained control, e.g., disabling `CC‑DEEP` for a specific generated file.

32. **Rust/Core — Implement BlacklistIncident professional report serializer**  
    Create `core/engine/src/blacklist_report.rs` with `BlacklistIncident` struct containing `incident_id`, `timestamp`, `detector_version`, `rule_id`, `location`, `excerpt`, and `recommended_action`. Implement `to_json() -> String` producing a `BLACKLIST‑REPORT‑1` object as defined in DQ‑49. Expose via a dedicated channel or embed in `ValidationResult` under `professional_report`.

33. **Policy/ALN — Implement ALN parser for blacklist‑spec files**  
    In `core/engine/src/policy.rs`, extend the ALN parser to recognize `kind: "blacklist‑spec"` sections. Parse `blacklist:` list items with fields `id`, `pattern`, `pattern_type`, `languages`, `context`, `severity`, and `message`. Map to `BlacklistRule` struct. Support YAML‑style frontmatter. Use this to load `specs/blacklist.aln` and `.ccblacklist.aln`.

34. **Policy/ALN — Implement recursive discovery and merging of .ccblacklist.aln**  
    Modify `Policy::load_specs` to walk the repository using `Navigator` and collect all `.ccblacklist.aln` files. For a given file path, build a list of policies along its ancestor directories. Merge rules with “most specific wins” by `id`: later (deeper) files override earlier ones. Store the merged `BlacklistProfile` per‑directory in a cache.

35. **Policy/ALN — Add profile‑specific blacklist sections in ALN**  
    Extend the blacklist‑spec grammar to support `profile <name>:` blocks, e.g., `profile github: blacklist: ...`. When building `BlacklistProfile`, start with `default` profile, then overlay the active profile’s rules (from SITQ `profile` field). This allows stricter rules for GitHub commits vs memory‑only sessions.

36. **Policy/ALN — Define and parse .ccpolicy.aln with blacklist_exemptions**  
    Extend the policy parser to recognize `blacklist_exemptions:` list with items `path_prefix` and `rule_ids`. Load these into `BlacklistExemption` structs. Store them in `BlacklistProfile` and apply during scanning to skip specific rules for exempted subtrees.

37. **Policy/ALN — Create specs/blacklist.aln canonical blacklist file**  
    Create file `./specs/blacklist.aln` with frontmatter `kind: "blacklist‑spec"`, `version: "1.0.0"`, and a `blacklist:` list containing default rules: `id: BL‑0001` for `"Rust Syn"`, `id: BL‑0002` for `"Tree‑Sitter"`, `id: BL‑0003` for `"(*/)"` contamination marker. Define `pattern_type: literal`, `languages: [any]`, `severity: block`. This serves as the global baseline.

38. **Policy/ALN — Create specs/wiring‑spec.aln with nodes and edges**  
    Create `./specs/wiring‑spec.aln` with `kind: "wiring‑spec"`, `version: "1.0.0"`. Define `nodes:` list for `Lib`, `Validator`, `Vfs`, `Navigator`, `TaskQueue`, `TokenWalker`. Define `required_edges:` and `forbidden_edges:` as per DQ‑60. This file is the reference for `WiringValidator`.

39. **Policy/ALN — Add cc_zero_forbidden_calls to invariants.aln**  
    In `./specs/invariants.aln`, under the `CC‑ZERO` section, add `forbidden_calls:` list with items like `std::env::var`, `std::env::var_os`. The policy parser should load these into a `HashSet<String>` used by `CCTokenWalker` for call‑pattern blacklisting in entry files.

40. **Policy/ALN — Define ALN grammar for BlacklistRule with pattern_type and context**  
    Extend the ALN parser to recognize `pattern_type` field with values `literal`, `glob`, `regex‑subset`. For `glob`, support `*` and `?` wildcards; for `regex‑subset`, support a restricted set (`.`, `*`, `+`, `?`, `|`, `[...]`). Recognize `context` field with values `import`, `declaration`, `any`. Map these to `BlacklistRule` fields.

41. **Policy/ALN — Implement glob pattern matching without external crates**  
    In `core/engine/src/blacklist_pattern.rs`, add `matches_glob(pattern: &str, text: &str) -> bool` using a recursive or iterative backtracking algorithm. Support `*` (any sequence) and `?` (exactly one character). Integrate this into `BlacklistPattern` when `pattern_type: "glob"` is used.

42. **Policy/ALN — Create per‑directory override example: core/experimental/.ccblacklist.aln**  
    Create example file `./core/experimental/.ccblacklist.aln` with frontmatter `kind: "blacklist‑spec"`, `scope: { path_prefix: "core/experimental", mode: "override" }`. Include rules that modify severity of `BL‑0003` to `report` and add new experimental‑only rule `BL‑0100` blocking `unsafe_experimental_api`. This tests the override mechanism.

43. **WASM/FFI — Expose cc_scan_blacklist as described in DQ‑56**  
    Ensure `cc_scan_blacklist(code: &str, profile_json: &str) -> String` is fully implemented and tested. Input `profile_json` must include `language` and optional `tags`. Output is a JSON array of `BlacklistMatch` objects with fields `rule_id`, `token`, `severity`, `line`, `column`, `exact_match`, `context`. This export is used by `github‑api.js` to pre‑screen files.

44. **WASM/FFI — Expose cc_get_wiring_json for external tooling**  
    Implement `cc_get_wiring_json() -> String` as specified. The returned JSON must match the compact schema (`nodes` with `id`/`role`, `edges` with `from`/`to`/`via`). Use this in CI to verify that the runtime wiring matches `wiring‑spec.aln`.

45. **WASM/FFI — Expose cc_poll_logs and cc_clear_logs**  
    Implement `cc_poll_logs() -> String` returning JSON array of pending `LogRecord` objects and clearing the buffer. Add `cc_clear_logs()` to discard logs without returning. JS will call `cc_poll_logs` on each animation frame or after each task to stream logs to `OutputPanel`.

46. **WASM/FFI — Expose cc_engine_id and cc_vfs_id**  
    Ensure `cc_engine_id() -> String` returns `"cc‑engine1"` and `cc_vfs_id() -> String` returns `"cc‑vfs1"`. These are used in reports and correlation IDs. The values must be stable across releases unless the engine version changes.

47. **WASM/FFI — Expose cc_check_blacklist for lightweight file screening**  
    Implement `cc_check_blacklist(path: &str, content: &str) -> String` that runs only Tier‑1 blacklist scan (hard markers and literal rules) and returns `{"ok": true}` or `{"ok": false, "reason": "...", "rule_id": "..."}`. This is called by `github‑api.js` before exposing file content to UI, allowing early rejection of contaminated files.

48. **WASM/FFI — Add correlation_id propagation through all exports**  
    Modify `cc_validate_code`, `cc_execute_task`, `cc_scan_blacklist` to accept an optional `correlation_id` parameter (or embed in metadata). Store this ID in `ValidationRequest` and `TaskQueuePayload`. Include it in all `LogRecord`, `ValidationResult`, `TaskReport`, and `BlacklistIncident` outputs. This enables end‑to‑end tracing.

49. **WASM/FFI — Define extern "C" surface for CCTokenWalker cross‑language testing**  
    Create `cpp/ctokenwalker.h` with functions `cctw_new`, `cctw_free`, `cctw_collect_symbols`, `cctw_collect_imports`, `cctw_scan_blacklist`. Implement these in C/C++ to mirror Rust `CCTokenWalker` behavior. The Rust engine can optionally call these via FFI for equivalence testing (gated by a feature flag).

50. **WASM/FFI — Link C++ path sanitizer as static library and call from Rust**  
    In `build.rs`, add directives to link `cpp/fallback/libccpath.a`. In `core/engine/src/path.rs`, declare `extern "C" { fn cc_sanitize_path(...) }` and wrap in safe Rust function. Use this in `PathCanonicalizer` chain as fallback. Ensure the C++ code is compiled with `‑O2` and does not allocate across FFI boundary.

51. **WASM/FFI — Add cc_bench_token_walker with blacklist metrics**  
    Update `cc_bench_token_walker` to accept `want_blacklist` flag and return extended metrics. Use this in `js/app/bench/token‑walker‑bench.js` to measure and display blacklist scanning overhead in the UI’s debug panel.

52. **WASM/FFI — Implement cc_vfs_snapshot_bytes for efficient snapshot transfer**  
    Add `cc_vfs_snapshot_bytes() -> Vec<u8>` returning UTF‑8 bytes of the VFS snapshot JSON. Use `thread_local!` buffer to avoid allocation per call. JS can receive this as `Uint8Array` and decode with `TextDecoder`, reducing string copy overhead.

53. **JS/UI — Implement minimal event bus module**  
    Create `js/app/core/event‑bus.js` with `on(event, callback)`, `off(event, callback)`, `emit(event, payload)`. Use a `Map` of event names to `Set` of callbacks. Ensure `emit` copies the listener set before iteration to allow safe removal during emission. Export a singleton instance.

54. **JS/UI — Wire FileTree to subscribe to vfs:updated events**  
    In `js/app/editor/file‑tree.js`, in the constructor, call `eventBus.on('vfs:updated', this.onVfsUpdated.bind(this))`. Implement `onVfsUpdated(payload)` to iterate `payload.changes` and call `this.model.applyWrite` or `this.model.applyDelete`. Then call `this.refreshDom()` to update the tree. Ensure only affected paths are re‑rendered.

55. **JS/UI — Implement VfsTreeModel with diff‑aware updates**  
    Create `js/app/editor/vfs‑tree‑model.js` with `VfsTreeModel` class. Maintain `nodesByPath` map and `rootChildren` array. Provide `applyWrite(path, isDir, sha)` that creates/updates node and attaches to parent. Provide `applyDelete(path)` that removes node and updates parent’s children. `rootChildren` is recomputed lazily. This minimizes DOM churn.

56. **JS/UI — Render ContaminationReport in OutputPanel with distinct style**  
    In `js/app/terminal/output‑panel.js`, add `renderContamination(report)` method. Create a `div` with CSS class `cc‑contamination‑report`, an icon `!`, title “Blacklist violation detected”, body showing pattern/severity/location, and a `<pre>` block for `surrounding_context`. Apply dark red background and red left border. Call this for each entry in `ValidationResult.contaminations`.

57. **JS/UI — Render professional blacklist report from TaskReport**  
    When `TaskReport` contains a `professional_report` field (or when `cc_execute_task` returns a dedicated blacklist response), extract the `BLACKLIST‑REPORT‑1` object. Display a modal or top‑banner with sections: summary, detection details, impact, actions taken, and guidance. Use the same styling as `ContaminationReport` but with more detail.

58. **JS/UI — Implement worker postMessage request‑response with correlation IDs**  
    Create `js/app/worker/cc‑agent‑worker.js` and `js/app/main‑worker‑bridge.js`. Main thread sends `{ type: "request", correlationId, op, payload }`. Worker calls appropriate WASM function and replies `{ type: "response", correlationId, ok, result }`. Main thread matches `correlationId` to pending Promise. Worker also sends unsolicited `event` and `log` messages.

59. **JS/UI — Stream worker logs to OutputPanel via cc_poll_logs**  
    In the worker, after each WASM call, call `cc_poll_logs()` and post any new logs to main thread as `{ type: "log", entries }`. In main thread, dispatch each entry to `eventBus.emit('log:record', entry)`. `OutputPanel` subscribes to `log:record` and appends formatted log lines, grouping by `correlation_id`.

60. **JS/UI — Implement “Fix CC‑SOV” button flow**  
    In `OutputPanel`, for each `CC‑SOV` failure that includes `suggested_replacement`, render a “Fix” button. On click, read current file content, apply text patch (replace old import with replacement), construct a SITQ `writefile` task payload, and call `cc_execute_task`. On success, update editor buffer and emit `vfs:updated` event.

61. **JS/UI — Add validation summary grouping by file and tag**  
    In `OutputPanel.renderValidation`, after receiving `ValidationResult`, group failures by `path`. For each file, create a collapsible section with summary counts. Inside, group failures by `tag`. Each failure entry displays message and a link that calls `eventBus.emit('validation:navigate', { path, line, column })`. Main UI listens and opens the file at the specified location.

62. **JS/UI — Implement validation:navigate event handling in main.js**  
    In `js/app/main.js`, subscribe to `validation:navigate`. When received, call `fileTree.selectPath(path)` (if exists), load file content via `ccReadFile`, set editor value, and move cursor to `line, column`. Highlight the offending line temporarily.

63. **JS/UI — Add keyboard shortcut to run “validate all”**  
    Bind a key combination (e.g., `Ctrl+Shift+V`) to trigger `validateAll()`. This function uses `Navigator` discovery (via `ccListDir` recursively) to gather all source files, builds a SITQ payload with `validateonly` tasks for each file, and calls `cc_execute_task`. Display aggregated results in `OutputPanel` with grouping.

64. **JS/UI — Display wiring metrics in diagnostics panel**  
    Create a small diagnostics view (e.g., under a “?” menu) that calls `cc_get_wiring_json()` and renders a simple canvas‑based graph of modules and edges. Also show wiring efficiency metric (cross‑module call count for last `validateonly` task) by calling `cc_get_wiring_metrics()` (to be implemented).

65. **JS/UI — Implement file tree context menu with “Scan for blacklist”**  
    Add a right‑click context menu on file tree items. Include option “Scan for blacklist” that calls `cc_scan_blacklist` on the selected file’s content. Display results in a popup or in `OutputPanel` as a temporary report.

66. **JS/UI — Add IndexedDB caching for GitHub API responses**  
    In `js/app/github‑api.js`, use `Cache` module (already planned) to store fetched trees and file contents in IndexedDB. Key by `cacheKey` derived from owner, repo, ref, and profile. Before fetching, check cache; if present and SHA matches, use cached data. On write, update cache entry. This reduces GitHub API calls.

67. **JS/UI — Implement exponential backoff and request queuing in github‑api.js**  
    Add a request queue with `maxConcurrent = 2`. For each GitHub API call, enqueue a function that executes `fetch`. On rate limit (403/429) or 5xx, increase `backoffMs` exponentially up to 60s. Delay queue processing until `Date.now() >= nextAllowedAt`. No external libraries; use `setTimeout` and `Promise`.

68. **JS/UI — Display blacklist summary report after SITQ run**  
    After `cc_execute_task` completes, if `TaskReport` contains `blacklist_summary`, render a dedicated section in `OutputPanel` showing total violations, severity counts, per‑rule counts, and per‑file breakdown. Use collapsible tables.

69. **Connector/Sandbox — Create connectors/chatgpt/sandbox.aln**  
    Create file `./connectors/chatgpt/sandbox.aln` with `kind: "connector‑sandbox"`, `platform: "chatgpt"`. Define `allowed_apis: ["fetch:https://api.github.com", "ccInitVfs", "ccValidateCode", "ccExecuteTask", "console.log"]`. Define `banned_globals: ["eval", "Function", "import()", "WebSocket", "window", "document", ...]`. Set `max_runtime_ms: 2000`. This file is used by the validator to enforce sandbox on connector code.

70. **Connector/Sandbox — Implement sandbox validation for connector adapter files**  
    In `Validator`, when validating a `.js` file under `connectors/`, load the corresponding `sandbox.aln`. Run a `CC‑SOV`‑style scan to detect usage of `banned_globals` (e.g., `window`, `eval`). Also scan for `fetch` calls that do not match `allowed_apis` patterns. Fail validation with tag `CC‑SANDBOX` if violations found.

71. **Connector/Sandbox — Create connector manifest: connectors/chatgpt/manifest.aln**  
    Create `./connectors/chatgpt/manifest.aln` with `kind: "connector‑manifest"`, `platform: "chatgpt"`, `entry_js: "./connectors/chatgpt/adapter.js"`, `policy: "./connectors/chatgpt/policy.aln"`. Include `capabilities: ["codegen", "validation", "vfs_read", "vfs_write", "tasks"]`. This manifest is read by the UI to discover available connectors.

72. **Connector/Sandbox — Write connector adapter template: connectors/chatgpt/adapter.js**  
    Create `./connectors/chatgpt/adapter.js` with exported functions `init(options)`, `fetchRepo(request)`, `sendResult(request)`. Implement `fetchRepo` to call GitHub API and return a VFS‑SNAPSHOT‑1 JSON. Implement `sendResult` to interpret `TaskReport` and push changes back to GitHub. Ensure code uses only sandbox‑allowed APIs.

73. **Connector/Sandbox — Add requested_capabilities section to sandbox.aln**  
    Extend `sandbox.aln` schema to include optional `requested_capabilities:` list, e.g., `network: { http: ["any"] }` or `realtime: ["websocket"]`. The engine compares these against a global allow‑list (configurable). If denied, connector fails to load. This allows privileged connectors with explicit permission.

74. **Testing/CI — Write unit tests for BlacklistCache validity logic**  
    In `core/engine/src/blacklist_cache.rs`, add `#[cfg(test)]` mod with tests: (1) cache hit when hash and version match and TTL not expired; (2) cache miss when hash differs; (3) cache miss when version differs; (4) cache miss when TTL expired. Use mock timestamps.

75. **Testing/CI — Write integration tests for PathCanonicalizer chain**  
    Create test file `tests/path_canonicalizer_tests.rs`. Test pure Rust normalization with various inputs (`.`, `..`, `//`, `\\`). Test fallback to C++ when Rust fails (use a mock FFI). Ensure paths escaping root are rejected. Ensure blacklisted directory names cause rejection.

76. **Testing/CI — Add wiring validation to GitHub Actions workflow**  
    In `.github/workflows/cc‑invariants.yml`, add a step that runs a Node script calling `cc_get_wiring_json()` and comparing against `specs/wiring‑spec.aln` using `WiringValidator` (or a JS port). Fail the workflow if any mismatch is found. This ensures architecture drift is caught in PRs.

77. **Testing/CI — Add blacklist pre‑commit hook to GitHub Actions**  
    Create a separate workflow `.github/workflows/cc‑blacklist.yml` that, on pull request, enumerates changed files, runs `cc_scan_blacklist` on each, and fails if any block‑level match is found. Output a machine‑readable JSON report for CI annotations.

78. **Testing/CI — Write conformance tests for CCTokenWalker cross‑language equivalence**  
    Create test corpus in `.conformance/token‑walker/` with pairs of Rust and C source files. Implement a test harness that runs Rust `CCTokenWalker` and C `cctw_*` functions, compares symbol and import sets, and asserts equality. Run in CI to ensure both implementations stay aligned.

79. **Testing/CI — Write tests for BlacklistPattern regex subset engine**  
    In `core/engine/src/blacklist_pattern.rs`, add tests covering literal matching, `.`, `*`, `+`, `?`, `|`, character classes, and nested quantifiers. Ensure edge cases (empty input, zero‑width matches) are handled correctly. Use these tests to validate the custom NFA engine.

80. **Testing/CI — Write tests for per‑directory blacklist override merging**  
    Create a test fixture with multiple `.ccblacklist.aln` files at different depths. Call the policy merger and assert that the final `BlacklistProfile` for a given path contains the correct rules with overridden severities and added rules. Test both `override` and `extend` modes.

81. **Testing/CI — Write tests for sandbox enforcement on connector JS**  
    Create a test connector file with banned globals (`eval`, `window`). Run `Validator::run_validation` with sandbox policy active. Assert that `CC‑SANDBOX` failures are emitted. Test that allowed APIs (`fetch` to GitHub) do not cause failures.

82. **Testing/CI — Write tests for TaskQueue transactional rollback**  
    In `tests/taskqueue_tests.rs`, create a SITQ payload with two tasks: first a valid write, second a validation‑failing write. Assert that after execution, VFS is unchanged (rollback occurred) and `TaskReport.ok` is false. Test nested transactions with `begin_tx`/`commit_tx`.

83. **Testing/CI — Write tests for cc_scan_blacklist export**  
    Create a Node.js test that calls `cc_scan_blacklist` with sample code containing banned tokens. Verify JSON output contains expected `BlacklistMatch` fields. Test that false positives in comments are avoided when context tracking is active.

84. **Testing/CI — Write tests for event bus in JS**  
    Create `js/app/core/__tests__/event‑bus.test.js` (using a simple test runner). Test `on`, `off`, `emit` semantics, including multiple listeners, removal during emission, and once‑only patterns. Ensure no memory leaks.

85. **Testing/CI — Write tests for VfsTreeModel diffing**  
    In `js/app/editor/__tests__/vfs‑tree‑model.test.js`, test `applyWrite` creates nodes, updates existing, and attaches to parent. Test `applyDelete` removes nodes and updates parent’s children. Test `rootChildren` sorting.

86. **Testing/CI — Write end‑to‑end test for “Fix CC‑SOV” flow**  
    Using a headless browser test (e.g., Playwright), simulate opening a file with a CC‑SOV violation, clicking “Fix” button, and verifying that the editor content is updated, the SITQ task succeeds, and the file tree reflects the change. Verify no blacklist contamination remains.

87. **Testing/CI — Add benchmark for blacklist scanning in CI**  
    Create a benchmark suite that runs `cc_bench_token_walker` with and without blacklist on a 10k‑line Rust file. Record `time_ms_with_blacklist` and assert it stays under a threshold (e.g., 10ms). Publish results as a CI artifact for trend analysis.

88. **Testing/CI — Write fuzz test for ALN blacklist parser**  
    Use a simple fuzzer (e.g., `cargo fuzz`) to generate random ALN‑like inputs and feed to the blacklist parser. Ensure no panics or crashes. This hardens the hand‑rolled parser.

89. **Documentation — Create specs/blacklist‑summary‑report.aln schema doc**  
    Write `./specs/blacklist‑summary‑report.aln` documenting the `BLACKLIST‑SUMMARY‑1` JSON schema with fields `total_violations`, `severity_counts`, `by_rule`, `by_file`. Include an example. This serves as the authoritative spec for report consumers.

90. **Documentation — Create specs/professional‑report.aln schema doc**  
    Write `./specs/professional‑report.aln` documenting the `BLACKLIST‑REPORT‑1` schema with fields `incident_id`, `timestamp`, `summary`, `detection`, `impact`, `actions_taken`, `recommended_action`, `guidance`, `context`. Provide example.

91. **Documentation — Update engine.aln with blacklist and wiring sections**  
    In `./specs/engine.aln`, add sections for `BlacklistProfile`, `BlacklistCache`, `WiringManifest`, `EventRouter`, and the new WASM exports (`cc_scan_blacklist`, `cc_get_wiring_json`, `cc_poll_logs`). Document their contracts and JSON shapes.

92. **Documentation — Create connector‑sandbox.aln schema doc**  
    Write `./specs/connector‑sandbox.aln` describing the `connector‑sandbox` ALN schema with `allowed_apis`, `banned_globals`, `max_runtime_ms`, and optional `requested_capabilities`. Provide examples.

93. **Refactor — Extract blacklist scanning logic from CCTokenWalker into dedicated module**  
    Move `scan_blacklist_segment`, context tracking, and rule matching from `token_walker.rs` into a new `blacklist_scanner.rs` module. `CCTokenWalker` calls into this module when `BIT_BLACKLIST` is set. This improves separation of concerns.

94. **Refactor — Replace ad‑hoc JSON strings in Validator with JsonBuilder**  
    Audit `Validator`, `TaskQueue`, and `Vfs` for any manual JSON string concatenation (e.g., using `format!`). Replace with `JsonBuilder` to ensure proper escaping and avoid syntax errors. This satisfies the requirement for robust JSON generation.

95. **Refactor — Unify path normalization across Rust and C++**  
    Ensure `cc_sanitize_path` in C++ follows exactly the same algorithm as Rust’s `normalize_path`. Write a conformance test that feeds identical inputs to both and asserts identical outputs. This guarantees consistency when fallback is used.

96. **Optimization — Pre‑compile hard blacklist markers into a DFA at build time**  
    Write a build script (`build.rs`) that reads `specs/blacklist.aln`, extracts literal markers, and generates a Rust file `blacklist_dfa.rs` containing a static transition table for Aho‑Corasick. Integrate this DFA into the Tier‑1 blacklist scanner for fast substring matching.

97. **Optimization — Cache BlacklistProfile per language and context**  
    In `BlacklistRuleSet`, pre‑compute `Vec<BlacklistRule>` slices for each `(RuleLanguage, RuleContext)` combination to avoid hash lookups in the hot loop. Use `once_cell::Lazy` or build at startup.

98. **Optimization — Reduce WASM boundary crossings by batching VFS reads**  
    Modify `cc_read_file` to accept an array of paths and return a JSON object mapping path to content. This reduces the number of WASM calls when hydrating multiple files (e.g., during “validate all”). Ensure the JS side batches requests.

99. **Optimization — Use Uint8Array for large snapshot transfers**  
    Ensure `cc_vfs_snapshot_bytes` returns a `Uint8Array` directly (via wasm‑bindgen) to avoid text decoding overhead. The JS side can parse JSON from the bytes using `TextDecoder`. This is more efficient for large snapshots.

100. **Polish — Ensure all user‑facing messages follow professional report guidelines**  
     Review all error messages, validation failures, and blacklist reports to ensure they are clear, actionable, and include recommended remediation steps. For blacklist incidents, always include the full professional report structure as defined, not just a plain string. This meets the (R) rule requirement for professional reporting.

---

*End of 100 Coding Tasks*  
*All tasks are traceable to the Code‑Command Blacklisting & Wiring Research Agenda (RQ‑201–OI‑30) and respect the sovereign stack constraints.*
