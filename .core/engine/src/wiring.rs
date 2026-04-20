// FILE .core/engine/src/wiring.rs
// core/engine/src/wiring.rs

// NOTE: This module wires together the core engine components (Vfs, Validator,
// TokenWalker, Navigator, TaskQueue) without any external dependencies.
// It is designed to be compatible with both native and WASM builds, and
// avoids global singletons by returning a fully‑constructed EngineGraph.

use std::fmt;

// These types are expected to live in their respective modules:
//
//   core/engine/src/vfs.rs          -> pub struct Vfs { .. }
//   core/engine/src/validator.rs    -> pub struct Validator { .. }
//   core/engine/src/token_walker.rs -> pub struct CcTokenWalker { .. }
//   core/engine/src/navigator.rs    -> pub struct Navigator { .. }
//   core/engine/src/task_queue.rs   -> pub struct TaskQueue { .. }
//
// and to provide the constructors and handles referenced below.

use crate::vfs::Vfs;
use crate::validator::Validator;
use crate::token_walker::CcTokenWalker;
use crate::navigator::Navigator;
use crate::task_queue::TaskQueue;

/// EngineGraph is the concrete, owned wiring of the core engine components.
///
/// In a WASM context this will typically be created once at startup and held
/// behind a static mutable pointer guarded by custom init logic; in a native
/// context, callers may own it directly or wrap it in Arc/Mutex as needed.
pub struct EngineGraph {
    pub vfs: Vfs,
    pub validator: Validator,
    pub token_walker: CcTokenWalker,
    pub navigator: Navigator,
    pub task_queue: TaskQueue,

    pub engine_id: &'static str, // "cc-engine1"
    pub vfs_id: &'static str,    // "cc-vfs1"
}

/// WiringManifest exposes borrowed references into an EngineGraph for
/// functions that should not own components but need access to them.
///
/// This keeps orchestration explicit: callers pass a manifest into
/// higher‑level operations instead of reaching for hidden globals.
pub struct WiringManifest<'a> {
    pub vfs: &'a mut Vfs,                 // cc-vfs, in-memory snapshot
    pub validator: &'a mut Validator,     // CC-VALIDATOR-1
    pub token_walker: &'a mut CcTokenWalker,
    pub navigator: &'a mut Navigator,     // CC-NAV walker over Vfs
    pub task_queue: &'a mut TaskQueue,    // CC-TASKQUEUE-1

    pub engine_id: &'a str,               // "cc-engine1"
    pub vfs_id: &'a str,                  // "cc-vfs1"
}

impl<'a> WiringManifest<'a> {
    /// Convenience constructor to derive a manifest from an EngineGraph.
    pub fn from_graph(graph: &'a mut EngineGraph) -> Self {
        WiringManifest {
            vfs: &mut graph.vfs,
            validator: &mut graph.validator,
            token_walker: &mut graph.token_walker,
            navigator: &mut graph.navigator,
            task_queue: &mut graph.task_queue,
            engine_id: graph.engine_id,
            vfs_id: graph.vfs_id,
        }
    }
}

#[derive(Debug)]
pub enum WiringError {
    VfsInitFailed,
    ValidatorInitFailed,
    NavigatorInitFailed,
    TaskQueueInitFailed,
    TokenWalkerInitFailed,
}

impl fmt::Display for WiringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WiringError::VfsInitFailed => write!(f, "VFS initialization failed"),
            WiringError::ValidatorInitFailed => write!(f, "Validator initialization failed"),
            WiringError::NavigatorInitFailed => write!(f, "Navigator initialization failed"),
            WiringError::TaskQueueInitFailed => write!(f, "TaskQueue initialization failed"),
            WiringError::TokenWalkerInitFailed => write!(f, "TokenWalker initialization failed"),
        }
    }
}

impl std::error::Error for WiringError {}

/// Constructs a fully‑wired EngineGraph from fresh component instances.
///
/// Each constructor is expected to be pure logic over in‑memory state:
/// no environment variables, no disk IO, and no network calls. This
/// keeps the wiring compatible with CC-ZERO and the WASM runtime.
pub fn connect_all() -> Result<EngineGraph, WiringError> {
    // cc-vfs in‑memory snapshot
    let vfs = Vfs::new().map_err(|_| WiringError::VfsInitFailed)?;

    // cc-token-walker core
    let token_walker = CcTokenWalker::new()
        .map_err(|_| WiringError::TokenWalkerInitFailed)?;

    // CC-VALIDATOR-1; validator owns an internal handle to the walker
    let validator = Validator::with_token_walker(token_walker)
        .map_err(|_| WiringError::ValidatorInitFailed)?;

    // CC-NAV, wired to the Vfs via a lightweight handle
    let navigator = Navigator::with_vfs_handle(vfs.clone_handle())
        .map_err(|_| WiringError::NavigatorInitFailed)?;

    // SITQ core; the queue will be given a WiringManifest at execution time
    let task_queue = TaskQueue::new().map_err(|_| WiringError::TaskQueueInitFailed)?;

    // The validator is the canonical owner of the token walker; callers
    // that need a direct reference can use token_walker_handle().
    let token_walker_handle = validator.token_walker_handle();

    Ok(EngineGraph {
        vfs,
        validator,
        token_walker: token_walker_handle,
        navigator,
        task_queue,
        engine_id: "cc-engine1",
        vfs_id: "cc-vfs1",
    })
}
