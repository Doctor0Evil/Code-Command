# Wiring and Integration Roadmap

This document outlines the concrete coding tasks for wiring the Code-Command engine components, making them easily discoverable and reusable by AI-Chat agents and coding tools.

## Overview

Three layers must be machine-discoverable:

1. **Core Components**: Vfs, Validator, TaskQueue, Navigator, TokenWalker, BlacklistProfile
2. **Wiring Descriptions**: Rust `WiringManifest` structs and ALN specs
3. **Integration Points**: Traits, helper functions, and manifest loaders

## Completed Tasks

### 1. Blacklist Engine Core (`core/engine/src/blacklist.rs`)

✅ **Implemented:**
- `BlacklistLoadError` enum for error handling
- `load_blacklist_profile(vfs: &Vfs)` - Canonical entry point to load and merge policies
- `parse_blacklist_aln(doc: &str)` - ALN parser for blacklist rules
- `scan_content(...)` - High-level scan entry point for Validator/TaskQueue
- Merge logic for global (`specs/blacklist.aln`) and local (`.ccblacklist.aln`) rules

**AI-Chat Usage Pattern:**
```rust
let profile = load_blacklist_profile(&vfs)?;
let validator = Validator::with_blacklist(..., profile);
let reports = scan_content(&profile, code, path, language, context);
```

### 2. JavaScript Event Bus (`js/app/event-bus.js`)

✅ **Implemented:**
- Simple pub/sub with `on()`, `off()`, `emit()`, `clear()`
- Standard event types documented as JSDoc typedefs:
  - `vfs:mounted`, `vfs:updated`
  - `file:opened`, `file:saved`
  - `validation:started`, `validation:completed`
  - `task:queued`, `task:completed`, `task:failed`

**AI-Chat Usage Pattern:**
```js
import { eventBus } from './event-bus.js';

eventBus.on('validation:completed', (event) => {
  console.log('Result:', event.result);
});

eventBus.emit({ type: 'file:opened', path: 'src/main.rs', content: '...' });
```

### 3. Engine Bootstrap Helper (`js/app/engine-bootstrap.js`)

✅ **Implemented:**
- `initEngineAndVfs(snapshotJson)` - Single entry point for engine initialization
- `executeTask(taskRequest)` - Task execution with automatic event emission
- `runValidation(validationRequest)` - Validation with correlation ID tracking
- WASM module lazy loading

**AI-Chat Usage Pattern:**
```js
import { initEngineAndVfs, runValidation } from './engine-bootstrap.js';

const { engineId, vfsId } = await initEngineAndVfs(snapshotJson);
const result = await runValidation({ code, path: 'src/main.rs', tags: ['CC-FULL'] });
```

## Remaining Tasks

### 4. Validator Builder Pattern

**Goal:** Replace ad-hoc `Validator::new()` calls with a builder that wires token walker, blacklist profile, and plugins.

**Tasks:**
- [ ] Create `core/engine/src/validator_builder.rs`
- [ ] Implement `ValidatorBuilder::new().with_blacklist(profile).with_plugin(plugin).build()`
- [ ] Update `wiring.rs::connect_all()` to use the builder
- [ ] Add `register_default_plugins(builder)` helper

### 5. Wiring Manifest Implementation

**Goal:** Make `connect_all()` the single authoritative wiring function.

**Tasks:**
- [ ] Define `WiringManifest` struct in `core/engine/src/wiring.rs`:
  ```rust
  pub struct WiringManifest {
      pub vfs: Vfs,
      pub validator: Validator,
      pub navigator: Navigator,
      pub taskqueue: TaskQueue,
  }
  ```
- [ ] Implement `connect_all() -> Result<WiringManifest, WiringError>`
- [ ] Integrate `WiringValidator` to check against `specs/wiring-spec.aln`
- [ ] Document in `specs/engine.aln`

### 6. ContaminationReport Integration

**Goal:** Ensure `ContaminationReport` flows through ValidationResult and SITQ TaskReport JSON.

**Tasks:**
- [ ] Verify `ContaminationReport` struct exists in `validator.rs`
- [ ] Add `contaminations: Vec<ContaminationReport>` field to `ValidationResult`
- [ ] Update JSON serializer to include contaminations
- [ ] Wire into `TaskReport` structure

### 7. Connector Sandbox Policy

**Goal:** Enforce allowed APIs and banned globals for connectors.

**Tasks:**
- [ ] Create `connectors/[platform]/sandbox.aln` schema
- [ ] Implement `SandboxPolicy` loader
- [ ] Add static checker for connector JS files
- [ ] Integrate with validator Tier-1 checks

## File Structure Reference

```
/workspace
├── core/engine/src/
│   ├── blacklist.rs          ✅ Complete
│   ├── blacklist_pattern.rs  ✅ Complete
│   ├── validator.rs          ⏳ Needs ContaminationReport integration
│   ├── validator_builder.rs  🔲 TODO
│   ├── wiring.rs             🔲 TODO
│   └── lib.rs                ⏳ Needs init_engine export
├── js/app/
│   ├── event-bus.js          ✅ Complete
│   └── engine-bootstrap.js   ✅ Complete
├── specs/
│   ├── blacklist.aln         🔲 Create sample
│   ├── wiring-spec.aln       🔲 Create spec
│   └── engine.aln            🔲 Create documentation
└── connectors/
    └── [platform]/
        └── sandbox.aln       🔲 TODO
```

## Next Steps for AI-Chat

When extending the engine, follow these patterns:

1. **To add a new blacklist rule**: Edit `specs/blacklist.aln` or `.ccblacklist.aln`
2. **To wire a new component**: Add to `WiringManifest` in `wiring.rs::connect_all()`
3. **To listen for events**: Use `eventBus.on(eventType, handler)` in JS
4. **To trigger validation**: Call `runValidation(request)` from `engine-bootstrap.js`

All wiring functions are designed to be zero-cost abstractions with clear entry points for automation.
