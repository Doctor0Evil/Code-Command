// FILE: ./policy/invariants/code-command-invariants.md
---
cc-invariants: [CC-VOL, CC-LANG, CC-CRATE, CC-FILE, CC-FULL, CC-DEEP, CC-ZERO, CC-PATH, CC-SOV, CC-NAV]
status: draft
---

# Code‑Command Invariant Tags

This document defines the **Code‑Command invariant tags** used as a shared policy surface between AI‑Chat platforms and GitHub‑based validators. Each tag represents a concrete, machine‑checkable rule derived from the Code‑Command (R) rules.

---

## CC-VOL — Volume Lock

**Intent:** Every Code‑Command response or artifact must contain a high volume of concrete code and automation, not minimal samples.

**Invariant:**

- The artifact MUST contain at least `N` distinct code units (for example: functions, structs, classes, or task definitions).
- The artifact MUST NOT be limited to stubbed or placeholder bodies only.
- Validators MAY enforce a configurable `N` (for example, `N >= 3` functions per artifact).

**Validation Sketch:**

- Scan for language‑specific function and struct signatures (`fn`, `function`, `class`, `struct`, etc.).
- Count distinct definitions.
- Fail CC‑VOL if the count is below the configured minimum.

---

## CC-LANG — Sovereign Stack

**Intent:** Code‑Command uses only the sovereign language stack: Rust, JavaScript, C++, and ALN/Markdown for policy, with Rust’s required TOML manifests allowed.

**Invariant:**

- Allowed source/policy extensions include:
  - `.rs`, `.js`, `.jsx` (if desired), `.cpp`, `.cc`, `.h`, `.hpp`, `.aln`, `.md`.
- Allowed **special** TOML files (for Rust tooling compatibility):
  - `Cargo.toml` at package or workspace roots.
  - Optionally `Cargo.lock` if present.
- Disallowed extensions include (but are not limited to):
  - `.py`, `.ts`, `.tsx`, `.go`, `.rb`, `.yaml`, `.yml`.
- Arbitrary `.toml` files (other than `Cargo.toml` / `Cargo.lock`) are discouraged unless explicitly documented as part of Code‑Command’s own config.

**Validation Sketch:**

- For each file:
  - If extension is one of the allowed source/policy extensions, accept.
  - If extension is `.toml` or `.lock`, only accept if filename is `Cargo.toml` or `Cargo.lock`.
  - Reject other extensions in a denylist (e.g., `.py`, `.ts`, `.tsx`, `.go`, `.rb`, `.yaml`, `.yml`).
- Optionally scan file content for signatures of unsupported languages to catch mis‑labeled files.
- Fail CC‑LANG if any unsupported extension or invalid `.toml` usage is detected.

---

## CC-CRATE — Fresh Logic

**Intent:** Every interaction must introduce new logic (crates, structs, configs, or functions), not just re‑explain existing code.

**Invariant:**

- Each artifact MUST introduce at least one new code construct:
  - New `struct`, `enum`, `class`, `fn`, `impl`, `mod`, or configuration block.
- Artifacts MUST NOT be purely commentary or policy with zero new programmatic capability when the intent is to deliver code.

**Validation Sketch:**

- Within a single artifact: verify presence of at least one definition keyword (`struct`, `enum`, `class`, `fn`, etc.).
- In a multi‑commit or multi‑file scenario: diff against the prior snapshot and ensure at least one new definition appears.
- Fail CC‑CRATE if no new constructs are introduced.

---

## CC-FILE — Destination Strict

**Intent:** Every code fragment is anchored to a concrete filename and path in the repository.

**Invariant:**

- Every code block or file MUST be preceded by a `FILE` header, such as:
  - `// FILE: ./src/core/agent.rs`
  - `// FILE: ./js/app/main.js`
  - `# FILE: ./policy/invariants/guide.aln` (for ALN/Markdown).
- The declared path MUST be non‑empty and MUST conform to CC‑PATH and CC‑DEEP.

**Validation Sketch:**

- Inspect the first `N` lines of the artifact for a line starting with `FILE:` (language‑appropriate comment syntax).
- Extract and normalize the path string.
- Fail CC‑FILE if no valid `FILE` header is found.

---

## CC-FULL — No Excerpts

**Intent:** Code‑Command does not emit incomplete code excerpts, samples, or illustrative fragments.

**Invariant:**

- Artifacts MUST NOT contain placeholders such as:
  - `...`
  - `// rest of code`
  - `/* omitted */`
  - `// TODO: implement the rest`
- Functions and modules MUST have complete, compilable or interpretable bodies (within reason for the target language).

**Validation Sketch:**

- Search for forbidden placeholder substrings.
- Optionally check for empty function bodies with comments only.
- Fail CC‑FULL if any placeholder or stub pattern is detected.

---

## CC-DEEP — Depth‑3 Struct

**Intent:** Key files are organized in at least three‑level deep directory structures for clarity and compatibility.

**Invariant:**

- Paths declared in `FILE` headers MUST have a directory depth of at least three components after normalization.
  - Valid example: `./src/core/agent.rs` → `["src", "core", "agent.rs"]` (depth 3).
  - Invalid example: `./main.rs` → `["main.rs"]` (depth 1).
- The depth requirement applies to code and core policy files, not necessarily to top‑level meta files like `README.md`.

**Validation Sketch:**

- Normalize each path (remove `.` and empty segments, split on `/`).
- Count path components.
- Fail CC‑DEEP if depth is below the configured minimum (default 3) for files that must obey this rule.

---

## CC-ZERO — Zero‑Setup Boot

**Intent:** The agent must run directly from a GitHub clone or static host, with no install or setup steps.

**Invariant:**

- Entry files (for example, `main.rs`, `index.js`, `index.html`, or WASM bootstrap JS) MUST:
  - Use only relative imports or module paths.
  - NOT reference `install`, `setup`, or similar bootstrap semantics.
  - NOT rely on OS‑specific temporary directories or installer paths.
- The system MUST be loadable from static files only (for example, GitHub Pages, raw GitHub, or equivalent).

**Validation Sketch:**

- Identify designated entry files (by convention or manifest).
- Scan for disallowed tokens (`install`, `setup`, `std::env::temp_dir`, etc.).
- Fail CC‑ZERO if any disallowed token or non‑relative import pattern is found.

---

## CC-PATH — Path Integrity

**Intent:** Paths must be clean, portable, and unambiguous.

**Invariant:**

- Paths MUST:
  - Use forward slashes (`/`) only.
  - Avoid double slashes in the middle of the path (`"//"`).
  - Avoid trailing slashes (except where explicitly representing a directory in metadata).
- Paths MUST NOT:
  - Use backslashes (`\`) as separators.
  - Contain merged filename instances or obvious typos.

**Validation Sketch:**

- For each path:
  - Reject if any `\` is present.
  - Reject if `//` appears (after normalization).
- Fail CC‑PATH if any malformed path is detected.

---

## CC-SOV — Sovereign Only (No Externals)

**Intent:** All functionality must be built from custom components in the allowed stack, with no external tools or libraries.

**Invariant:**

- Code MUST NOT:
  - Import or require external crates/libraries not defined within Code‑Command.
  - Reference external SaaS APIs or proprietary SDKs as dependencies.
- Code MAY:
  - Use standard libraries for Rust/JS/CPP.
  - Use internal modules (for example, `mod core`, `crate::invariants`, local `./` imports).

**Validation Sketch:**

- Scan `use`, `mod`, `require`, `import`, and `#include` statements.
- Maintain an allowlist of standard‑library and internal module prefixes.
- Fail CC‑SOV if any import falls outside the allowlist.

---

## CC-NAV — Custom Navigation & Mount Logic

**Intent:** Directory navigation, mounting, and file placement logic must be implemented with custom code, not generic utilities.

**Invariant:**

- The codebase MUST contain at least one custom navigation/mounting function (for example, Rust functions using `std::fs::read_dir`, or JS functions using the GitHub API directly).
- Navigation MUST NOT rely on external tree‑walking libraries or generic project‑structure frameworks.
- Mount/unmount logic MUST be discoverable and readable as part of the Code‑Command core.

**Validation Sketch:**

- Search for functions with names or signatures matching navigation semantics (for example, `walk`, `mount`, `unmount`, `list_dir`, `scan_tree`).
- Confirm that they use standard library primitives or direct HTTP calls, not external navigation packages.
- Fail CC‑NAV if such custom logic is absent or replaced by external tools.

---
