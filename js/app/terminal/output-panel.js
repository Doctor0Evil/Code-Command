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
    this.logsVisible = false;
    this.logEntries = [];
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

  /**
   * Toggle the logs view visibility. [file:2]
   */
  toggleLogs() {
    this.logsVisible = !this.logsVisible;
    if (this.logsVisible) {
      this.refreshLogs();
    } else {
      // Return to normal terminal view
      this.el.innerHTML = '';
      for (const line of this.lines) {
        const lineEl = document.createElement("div");
        lineEl.style.whiteSpace = "pre-wrap";
        lineEl.textContent = line.text;
        if (line.cssClass) {
          lineEl.classList.add(line.cssClass);
        }
        this.el.appendChild(lineEl);
      }
    }
  }

  /**
   * Refresh the logs display from WASM. [file:2]
   */
  async refreshLogs() {
    if (!this.logsVisible) return;

    try {
      // Call WASM to get logs
      if (typeof window.CCEngine !== 'undefined' && window.CCEngine.cc_get_logs) {
        const logsJson = window.CCEngine.cc_get_logs();
        this.logEntries = JSON.parse(logsJson || '[]');
      }

      this.el.innerHTML = '';
      
      // Add header with controls
      const headerEl = document.createElement('div');
      headerEl.style.marginBottom = '8px';
      headerEl.style.paddingBottom = '8px';
      headerEl.style.borderBottom = '1px solid #1f2937';
      headerEl.innerHTML = `
        <span style="color: #60a5fa; font-weight: bold;">System Logs</span>
        <span style="color: #6b7280; margin-left: 12px;">(${this.logEntries.length} entries)</span>
        <button id="cc-copy-logs" style="margin-left: 12px; background: #020617; border: 1px solid #1f2937; color: #e5e7eb; padding: 2px 8px; border-radius: 4px; cursor: pointer; font-size: 11px;">Copy Logs</button>
        <button id="cc-clear-logs" style="margin-left: 8px; background: #020617; border: 1px solid #1f2937; color: #e5e7eb; padding: 2px 8px; border-radius: 4px; cursor: pointer; font-size: 11px;">Clear</button>
      `;
      this.el.appendChild(headerEl);

      // Filter controls
      const filterEl = document.createElement('div');
      filterEl.style.marginBottom = '8px';
      filterEl.innerHTML = `
        <label style="color: #9ca3af; font-size: 11px; margin-right: 8px;">Filter:</label>
        <label style="color: #60a5fa; font-size: 11px; margin-right: 8px;"><input type="checkbox" data-level="INFO" checked> INFO</label>
        <label style="color: #facc15; font-size: 11px; margin-right: 8px;"><input type="checkbox" data-level="WARN" checked> WARN</label>
        <label style="color: #f97373; font-size: 11px; margin-right: 8px;"><input type="checkbox" data-level="ERROR" checked> ERROR</label>
        <label style="color: #9ca3af; font-size: 11px; margin-right: 8px;"><input type="checkbox" data-level="DEBUG"> DEBUG</label>
      `;
      this.el.appendChild(filterEl);

      // Wire up filter checkboxes
      filterEl.querySelectorAll('input[type="checkbox"]').forEach(cb => {
        cb.addEventListener('change', () => this.refreshLogs());
      });

      // Wire up copy button
      const copyBtn = headerEl.querySelector('#cc-copy-logs');
      if (copyBtn) {
        copyBtn.addEventListener('click', () => this.copyLogs());
      }

      // Wire up clear button
      const clearBtn = headerEl.querySelector('#cc-clear-logs');
      if (clearBtn) {
        clearBtn.addEventListener('click', () => {
          if (typeof window.CCEngine !== 'undefined' && window.CCEngine.cc_clear_logs) {
            window.CCEngine.cc_clear_logs();
            this.refreshLogs();
          }
        });
      }

      // Get selected levels
      const selectedLevels = Array.from(filterEl.querySelectorAll('input[type="checkbox"]:checked'))
        .map(cb => cb.getAttribute('data-level'));

      // Render log entries
      const containerEl = document.createElement('div');
      containerEl.style.maxHeight = 'calc(100% - 80px)';
      containerEl.style.overflow = 'auto';

      for (const entry of this.logEntries) {
        if (!selectedLevels.includes(entry.level)) continue;

        const logLine = document.createElement('div');
        logLine.style.fontFamily = 'ui-monospace, monospace';
        logLine.style.fontSize = '11px';
        logLine.style.padding = '2px 0';
        
        const timeStr = entry.timestamp ? new Date(entry.timestamp).toISOString() : '---';
        const levelColor = {
          'INFO': '#60a5fa',
          'WARN': '#facc15',
          'ERROR': '#f97373',
          'DEBUG': '#9ca3af'
        }[entry.level] || '#e5e7eb';

        logLine.innerHTML = `
          <span style="color: #6b7280;">[${timeStr}]</span>
          <span style="color: ${levelColor}; font-weight: bold; margin: 0 8px;">[${entry.level}]</span>
          <span style="color: #4ade80;">${entry.module}:</span>
          <span style="color: #e5e7eb;">${entry.message}</span>
        `;
        containerEl.appendChild(logLine);
      }

      this.el.appendChild(containerEl);
      this.el.scrollTop = this.el.scrollHeight;
    } catch (err) {
      console.error('Failed to refresh logs:', err);
      this.logError('Failed to load logs.');
    }
  }

  /**
   * Copy all logs to clipboard. [file:2]
   */
  async copyLogs() {
    try {
      const logsText = this.logEntries.map(entry => {
        const timeStr = entry.timestamp ? new Date(entry.timestamp).toISOString() : '---';
        return `[${timeStr}] [${entry.level}] ${entry.module}: ${entry.message}`;
      }).join('\n');

      await navigator.clipboard.writeText(logsText);
      this.logOk('Logs copied to clipboard.');
    } catch (err) {
      this.logError('Failed to copy logs.');
    }
  }

  appendLine(text, cssClass) {
    if (!this.el) return;

    // Store line for potential restoration
    this.lines.push({ text, cssClass });

    // Don't render if logs view is active
    if (this.logsVisible) return;

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
