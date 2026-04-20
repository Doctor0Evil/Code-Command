// FILE: ./js/app/panic-channel.js
import OutputPanel from "./terminal/output-panel.js";

export function handlePanicReport(jsonText) {
  let report;
  try {
    report = JSON.parse(jsonText);
  } catch {
    report = { tag: "CC-PANIC", severity: "fatal", file: "", line: 0, message: jsonText };
  }

  const vr = {
    ok: false,
    tag: "CC-PANIC",
    failures: [
      `CC-PANIC ${report.file}:${report.line} ${report.message}`,
    ],
  };

  // Option 1: surface via OutputPanel only.
  const panel = window.CodeCommandOutputPanel;
  if (panel) {
    panel.logError(`PANIC in engine: ${report.message}`);
  }

  // Option 2: attach into a synthetic ValidationResult table keyed by "<engine>".
  window.CodeCommandLastPanicValidation = vr;
}
