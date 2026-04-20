// FILE: ./js/app/main-worker-bridge.js
let agentWorker = null;
let pending = Object.create(null); // id -> { resolve, reject }

export function initAgentWorker() {
  agentWorker = new Worker("./js/app/worker/cc-agent-worker.js", { type: "module" });
  agentWorker.onmessage = (ev) => {
    const msg = ev.data;
    if (!msg || typeof msg !== "object") return;

    if (msg.direction === "worker->main") {
      if (msg.type === "cc-execute-task") {
        const resultJson = CCEngine.ccexecutetask(msg.taskJson); // synchronous WASM call.[file:2]
        agentWorker.postMessage({
          direction: "main->worker",
          type: "cc-result",
          id: msg.id,
          ok: true,
          payload: resultJson,
        });
      } else if (msg.type === "cc-validate") {
        const resultJson = CCEngine.ccvalidate(msg.code, msg.tagsJson);[file:2]
        agentWorker.postMessage({
          direction: "main->worker",
          type: "cc-result",
          id: msg.id,
          ok: true,
          payload: resultJson,
        });
      }
    } else if (msg.direction === "worker->ui") {
      const slot = pending[msg.id];
      if (!slot) return;
      delete pending[msg.id];
      if (msg.ok) slot.resolve(msg.payload);
      else slot.reject(new Error("Worker reported failure"));
    }
  };
}

export function runTaskViaWorker(taskJson) {
  return new Promise((resolve, reject) => {
    const id = Date.now() ^ Math.floor(Math.random() * 1e6);
    pending[id] = { resolve, reject };
    agentWorker.postMessage({
      kind: "runTask",
      taskJson,
      id,
    });
  });
}
