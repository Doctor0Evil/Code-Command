// FILE: ./js/app/terminal/output-panel.js

// Read-only terminal emulator for Code-Command. Renders messages from the
// WASM engine and UI with simple ANSI-like styling into a <pre> element. [file:2]

export class OutputPanel {
  /**
   * @param {HTMLPreElement} element
   */
  constructor(element) {
    this.el = element;
    this.lines = [];
  }

  logInfo(message) {
    this.appendLine(message, "cc-ansi-info");
  }

  logWarn(message) {
    this.appendLine(message, "cc-ansi-warn");
  }

  logError(message) {
    this.appendLine(message, "cc-ansi-error");
  }

  logOk(message) {
    this.appendLine(message, "cc-ansi-ok");
  }

  /**
   * Generic entry point for WASM println!-style hooks.
   * Accepts raw text with optional ANSI-like tags and renders it. [file:2]
   */
  logRaw(message) {
    this.appendLine(message, null);
  }

  /**
   * Renders a task report JSON string produced by cc_execute_task. [file:2]
   * @param {string} reportJson
   */
  renderTaskReport(reportJson) {
    try {
      const report = JSON.parse(reportJson);
      const status = report.ok ? "OK" : "FAILED";
      const statusClass = report.ok ? "cc-ansi-ok" : "cc-ansi-error";
      this.appendLine(`TaskQueue: ${status}`, statusClass);

      if (Array.isArray(report.operations)) {
        for (const op of report.operations) {
          this.appendLine(`  • ${op}`, "cc-ansi-info");
        }
      }

      const validations = report.validations || {};
      for (const path of Object.keys(validations)) {
        const res = validations[path];
        const ok = !!res.ok;
        const cls = ok ? "cc-ansi-ok" : "cc-ansi-error";
        this.appendLine(`validate: ${path} => ${ok ? "ok" : "failed"}`, cls);

        if (Array.isArray(res.failures)) {
          for (const f of res.failures) {
            this.appendLine(`    - ${f}`, "cc-ansi-error");
          }
        }
      }
    } catch (err) {
      this.logError("Failed to parse task report.");
    }
  }

  appendLine(text, cssClass) {
    if (!this.el) return;

    const lineEl = document.createElement("div");
    lineEl.style.whiteSpace = "pre-wrap";
    lineEl.textContent = text;

    if (cssClass) {
      lineEl.classList.add(cssClass);
    } else {
      // Basic ANSI sequence parsing: turn markers like [INFO], [WARN], [ERR] into classes. [file:2]
      const marker = parseAnsiMarker(text);
      if (marker) {
        lineEl.classList.add(marker);
      }
    }

    this.el.appendChild(lineEl);
    this.el.scrollTop = this.el.scrollHeight;
  }
}

/* ---------- Minimal ANSI-like Marker Parser ---------- */

function parseAnsiMarker(text) {
  if (text.startsWith("[INFO]")) return "cc-ansi-info";
  if (text.startsWith("[WARN]")) return "cc-ansi-warn";
  if (text.startsWith("[ERROR]")) return "cc-ansi-error";
  if (text.startsWith("[OK]")) return "cc-ansi-ok";
  return null;
}
