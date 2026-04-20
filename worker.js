// worker.js
self.onmessage = async (event) => {
  const msg = event.data;
  if (msg.type !== "request") return;

  const { correlationId, op, payload } = msg;
  try {
    let result;
    if (op === "validate") {
      result = await runValidate(payload); // calls WASM ccValidateCode
    } else if (op === "executeTask") {
      result = await runExecuteTask(payload); // calls WASM ccExecuteTask
    }
    self.postMessage({
      type: "response",
      correlationId,
      ok: true,
      result
    });
  } catch (err) {
    self.postMessage({
      type: "response",
      correlationId,
      ok: false,
      error: String(err)
    });
  }
};
