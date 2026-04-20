// FILE .core/engine/src/wiring.rs

pub struct EngineGraph {
    pub validator: Validator,
    pub vfs: Vfs,
    pub navigator: Navigator,
    pub task_queue: TaskQueue,
    pub token_walker: TokenWalker,
}

#[derive(Debug)]
pub enum WiringError {
    VfsInitFailed,
    ValidatorInitFailed,
    NavigatorInitFailed,
    TaskQueueInitFailed,
    TokenWalkerInitFailed,
}

pub fn connect_all() -> Result<EngineGraph, WiringError> {
    // Each constructor returns an owned value; there are no shared singletons here.
    let vfs = Vfs::new().map_err(|_| WiringError::VfsInitFailed)?;
    let token_walker = TokenWalker::new();                 // cc‑token‑walker core.[file:4]
    let validator = Validator::with_token_walker(token_walker)
        .map_err(|_| WiringError::ValidatorInitFailed)?;
    let navigator = Navigator::with_vfs_handle(vfs.clone_handle())
        .map_err(|_| WiringError::NavigatorInitFailed)?;
    let task_queue = TaskQueue::new();                     // SITQ core.[file:2][file:4]

    Ok(EngineGraph {
        validator,
        vfs,
        navigator,
        task_queue,
        token_walker: validator.token_walker_handle(),
    })
}
