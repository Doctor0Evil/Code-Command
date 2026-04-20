// FILE: ./js/app/worker/cc-agent-worker.js
// Runs in a dedicated Worker context, no DOM, no WASM.

let nextId = 1;

self.onmessage = function (ev) {
  const msg = ev.data;
  if (!msg || typeof msg !== "object") return;

  if (msg.kind === "runTask") {
    const id = nextId++;
    // Forward to main thread's engine host:
    self.postMessage({
      direction: "worker->main",
      type: "cc-execute-task",
      id,
      taskJson: msg.taskJson,
    });
  } else if (msg.kind === "validate") {
    const id = nextId++;
    self.postMessage({
      direction: "worker->main",
      type: "cc-validate",
      id,
      code: msg.code,
      tagsJson: msg.tagsJson,
    });
  } else if (msg.direction === "main->worker" && msg.type === "cc-result") {
    // Engine replies from main thread, forward to UI-controller logic running in worker.
    // At minimum, just bounce back to the main thread UI shell if you want.
    self.postMessage({
      direction: "worker->ui",
      id: msg.id,
      ok: msg.ok,
      payload: msg.payload,
    });
  }
};
