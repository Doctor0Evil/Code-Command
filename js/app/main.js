// FILE: ./js/app/main.js

// Central browser entry for Code-Command.
// Loads WASM engine, Monaco editor, file tree, and output panel,
// wiring UI events to the CC-API. Satisfies CC-ZERO by using only
// relative imports and no environment/setup logic.

import initWasm, * as CCEngine from '../lib/wasm/ccengine.js';
import createEditor from './editor/monaco-loader.js';
import FileTree from './editor/file-tree.js';
import OutputPanel from './terminal/output-panel.js';
import * as GithubAPI from './github/api.js';

let editor = null;
let fileTree = null;
let outputPanel = null;
let currentRepo = null;

// WASM initialization state.
let wasmReady = false;

/**
 * VFS snapshot entry shape used by ccinitvfs:
 *
 * type VfsEntry = {
 *   path: string;     // normalized logical path, e.g. "core/engine/src/lib.rs"
 *   content: string;  // decoded UTF-8 text content, "" for directories
 *   sha: string;      // Git blob SHA or "" for synthetic entries
 *   isdir: boolean;   // true for directories, false for files
 * };
 *
 * The snapshot passed to ccinitvfs is: VfsEntry[]
 */

/**
 * Single-Iteration Task Queue payload shape used by ccexecutetask:
 *
 * type TaskKind = "create" | "update" | "validate";
 *
 * type Task = {
 *   kind: TaskKind;
 *   path: string;        // logical path, e.g. "src/main.rs"
 *   content?: string;    // new file content for create/update
 *   sha?: string;        // Git SHA for optimistic concurrency on write
 *   tags?: string[];     // CC- tag IDs for validation, e.g. ["CC-FILE","CC-FULL"]
 * };
 *
 * type TaskQueuePayload = {
 *   tasks: Task[];
 * };
 */

/**
 * Initialize the CC-Engine WASM module and UI components.
 * This runs once on page load and does not perform any setup beyond
 * fetching static assets, satisfying CC-ZERO.
 */
async function bootstrap() {
    outputPanel = new OutputPanel(
        document.getElementById('cc-terminal-output')
    );

    outputPanel.logInfo('Initializing Code-Command engine...');

    try {
        // Initialize WASM from generated glue; this fetches cc-engine.wasm.
        await initWasm();
        wasmReady = true;

        // Verify that the canonical cc-vfs implementation is active.
        const vfsId = CCEngine.ccvfs_id();
        if (vfsId !== 'cc-vfs:1') {
            outputPanel.logError(`Unexpected VFS identity: ${vfsId}`);
        } else {
            outputPanel.logOk(`VFS active: ${vfsId}`);
        }

        const editorContainer = document.getElementById('cc-editor');
        editor = await createEditor(
            editorContainer,
            '// Code-Command ready.\n',
            'rust'
        );

        const treeContainer = document.getElementById('cc-file-tree');
        fileTree = new FileTree(treeContainer, handleFileSelect);

        wireControls();

        outputPanel.logOk('Engine and UI initialized.');
    } catch (err) {
        console.error(err);
        if (outputPanel) {
            outputPanel.logError('Failed to initialize Code-Command engine.');
        }
    }
}

/**
 * Wire header controls (Load Repo, Run Task, Toggle Logs) to their handlers.
 */
function wireControls() {
    const loadButton = document.getElementById('cc-load-repo');
    const runTaskButton = document.getElementById('cc-run-task');
    const toggleLogsButton = document.getElementById('cc-toggle-logs');
    const repoInput = document.getElementById('cc-repo-input');

    loadButton.addEventListener('click', async () => {
        const value = (repoInput.value || '').trim();
        if (!value) {
            outputPanel.logWarn('Enter owner/repo before loading.');
            return;
        }
        const [owner, repo] = value.split('/');
        if (!owner || !repo) {
            outputPanel.logError('Repository must be in the form owner/repo.');
            return;
        }
        await loadRepository(owner, repo);
    });

    runTaskButton.addEventListener('click', async () => {
        await runCurrentTask();
    });

    if (toggleLogsButton) {
        toggleLogsButton.addEventListener('click', () => {
            outputPanel.toggleLogs();
        });
    }
}

/**
 * Load a GitHub repository tree into the VFS and update the file tree UI.
 */
async function loadRepository(owner, repo) {
    if (!wasmReady) {
        outputPanel.logError('WASM engine not ready.');
        return;
    }

    try {
        outputPanel.logInfo(`Loading repo ${owner}/${repo}...`);

        const tree = await GithubAPI.fetchRepoTree(owner, repo);
        const vfsSnapshot = GithubAPI.treeToVfsSnapshot(tree);

        // Seed the WASM-side VFS.
        CCEngine.ccinitvfs(JSON.stringify(vfsSnapshot));

        // Update UI tree.
        fileTree.setTree(tree);
        currentRepo = { owner, repo };

        outputPanel.logOk(`Repository loaded: ${owner}/${repo}`);
    } catch (err) {
        console.error(err);
        outputPanel.logError('Failed to load repository tree.');
    }
}

/**
 * Handles file selection from the FileTree component.
 * Load file content into the editor via ccreadfile.
 */
async function handleFileSelect(path) {
    if (!wasmReady) {
        outputPanel.logError('WASM engine not ready.');
        return;
    }

    try {
        outputPanel.logInfo(`Opening ${path}...`);

        // Ask WASM to read from VFS (which may call back to JS/GitHub).
        const content = CCEngine.ccreadfile(path);
        if (editor) {
            const language = guessLanguageFromPath(path);
            editor.setValue(content || '');
            if (window.monaco && editor.getModel()) {
                window.monaco.editor.setModelLanguage(
                    editor.getModel(),
                    language
                );
            }
        }

        outputPanel.logOk(`Opened ${path}.`);
    } catch (err) {
        console.error(err);
        outputPanel.logError(`Failed to read file: ${path}`);
    }
}

/**
 * Build a TaskQueuePayload for the current editor buffer and run it.
 *
 * Minimal two-task queue:
 *  - update the current file with the editor content
 *  - validate it with a basic CC-tag set
 */
async function runCurrentTask() {
    if (!wasmReady) {
        outputPanel.logError('WASM engine not ready.');
        return;
    }
    if (!editor) {
        outputPanel.logError('Editor not initialized.');
        return;
    }

    const model = editor.getModel();
    if (!model) {
        outputPanel.logError('No active editor model.');
        return;
    }

    const path = fileTree.getActivePath() || 'src/main.rs';
    const code = editor.getValue();

    /** @type {TaskQueuePayload} */
    const payload = {
        tasks: [
            {
                kind: 'update',
                path,
                content: code,
                sha: '',
                tags: [],
            },
            {
                kind: 'validate',
                path,
                content: '',
                sha: '',
                tags: ['CC-FILE', 'CC-LANG', 'CC-FULL', 'CC-PATH', 'CC-DEEP'],
            },
        ],
    };

    outputPanel.logInfo(
        `Running Single-Iteration Task Queue for ${path}...`
    );

    try {
        const reportJson = CCEngine.ccexecutetask(JSON.stringify(payload));
        renderTaskReport(reportJson);
    } catch (err) {
        console.error(err);
        outputPanel.logError('Task execution failed.');
    }
}

/**
 * Render the task queue report JSON in the terminal output panel.
 */
function renderTaskReport(reportJson) {
    try {
        const report = JSON.parse(reportJson);
        if (report.ok) {
            outputPanel.logOk('Task queue completed successfully.');
        } else {
            outputPanel.logError('Task queue completed with failures.');
        }

        if (Array.isArray(report.operations)) {
            for (const op of report.operations) {
                outputPanel.logInfo(op);
            }
        }

        if (report.validations) {
            Object.keys(report.validations).forEach((path) => {
                const res = report.validations[path];
                if (res.ok) {
                    outputPanel.logOk(`Validation OK: ${path}`);
                } else {
                    outputPanel.logError(
                        `Validation failed for ${path}: ${res.failures.join(
                            '; '
                        )}`
                    );
                }
            });
        }
    } catch (err) {
        console.error(err);
        outputPanel.logError('Invalid task report JSON.');
    }
}

/**
 * Heuristic language guesser based on file extension.
 */
function guessLanguageFromPath(path) {
    const lower = path.toLowerCase();
    if (lower.endsWith('.rs')) return 'rust';
    if (lower.endsWith('.js')) return 'javascript';
    if (
        lower.endsWith('.cpp') ||
        lower.endsWith('.cc') ||
        lower.endsWith('.h')
    ) {
        return 'cpp';
    }
    if (lower.endsWith('.md') || lower.endsWith('.aln')) return 'markdown';
    return 'plaintext';
}

// Kick everything off on DOM ready. No setup scripts, no env vars, satisfying CC-ZERO.
document.addEventListener('DOMContentLoaded', () => {
    bootstrap();
});
