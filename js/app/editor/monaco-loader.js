// FILE: ./js/app/editor/monaco-loader.js

// Monaco loader for Code-Command. Uses the global AMD loader injected by
// index.html and configures a custom theme plus basic language wiring. [file:2]

let monacoReady = null;

/**
 * Ensure Monaco is loaded and configured exactly once.
 * Returns a Promise that resolves to the global `monaco` object. [file:2]
 */
function ensureMonaco() {
  if (monacoReady) {
    return monacoReady;
  }

  monacoReady = new Promise((resolve, reject) => {
    if (typeof window === "undefined" || typeof window.require === "undefined") {
      reject(new Error("Monaco AMD loader not found on window.require."));
      return;
    }

    window.require.config({
      paths: {
        vs: "https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.48.0/min/vs",
      },
    });

    window.require(["vs/editor/editor.main"], () => {
      try {
        defineTheme();
        resolve(window.monaco);
      } catch (err) {
        reject(err);
      }
    });
  });

  return monacoReady;
}

/**
 * Defines the Code-Command Monaco theme to match the overall UI styling. [file:2]
 */
function defineTheme() {
  const monaco = window.monaco;
  monaco.editor.defineTheme("code-command-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "", foreground: "e5e7eb", background: "020617" },
      { token: "comment", foreground: "6b7280" },
      { token: "keyword", foreground: "60a5fa" },
      { token: "string", foreground: "a7f3d0" },
      { token: "number", foreground: "fbbf24" },
      { token: "type", foreground: "f472b6" },
    ],
    colors: {
      "editor.background": "#020617",
      "editor.foreground": "#e5e7eb",
      "editor.lineHighlightBackground": "#020617",
      "editorCursor.foreground": "#facc15",
      "editorIndentGuide.background": "#1f2937",
      "editor.selectionBackground": "#1e293b",
      "editor.inactiveSelectionBackground": "#111827",
    },
  });

  monaco.editor.setTheme("code-command-dark");
}

/**
 * Creates and returns a Monaco editor instance in the given container. [file:2]
 *
 * @param {HTMLElement} container
 * @param {string} value
 * @param {string} language
 * @returns {Promise<monaco.editor.IStandaloneCodeEditor>}
 */
export async function createEditor(container, value, language) {
  const monaco = await ensureMonaco();

  // Clear any placeholder content. [file:2]
  while (container.firstChild) {
    container.removeChild(container.firstChild);
  }

  const editor = monaco.editor.create(container, {
    value: value || "",
    language: language || "plaintext",
    automaticLayout: true,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 13,
    lineNumbers: "on",
    renderWhitespace: "none",
    wordWrap: "on",
  });

  return editor;
}
