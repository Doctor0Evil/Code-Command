/**
 * AI Command Compiler for Code-Command
 * 
 * Compiles line-oriented command grammar into SITQ payloads.
 * Supports Tasks A–E with CC-tag presets from cc-policy.aln.
 * 
 * Command Grammar:
 *   VERSION 1
 *   TASK_A <path>                    - Normalize paths & inject FILE headers
 *   TASK_B <path> [Vmin=N]           - Enforce CC-FULL & CC-VOL
 *   TASK_C <path>                    - Wiring graph audit
 *   TASK_D <path>                    - Blacklist scan
 *   TASK_E <platform>                - Connector adapter scaffolding
 * 
 * @module app/commands/ai-command-compiler
 */

import { cclistdir, ccexecutetask } from '../wasm-bridge.js';

// ============================================================================
// TAG PRESETS (from cc-policy.aln)
// ============================================================================

const TAG_PRESETS = {
  TASK_A: {
    validateonly: ['CC-FILE', 'CC-PATH', 'CC-DEEP', 'CC-LANG', 'CC-FULL', 'CC-SOV'],
    writefile: ['CC-FILE', 'CC-PATH', 'CC-DEEP', 'CC-LANG', 'CC-FULL', 'CC-SOV']
  },
  TASK_B: {
    validateonly: ['CC-FULL', 'CC-VOL', 'CC-LANG'],
    writefile: ['CC-FULL', 'CC-VOL', 'CC-LANG', 'CC-SOV']
  },
  TASK_C: {
    validateonly: ['CC-FULL', 'CC-PATH', 'CC-SOV', 'CC-DEEP']
  },
  TASK_D: {
    validateonly: ['CC-FULL', 'CC-LANG'],
    blacklistEnabled: true
  },
  TASK_E: {
    writefile: ['CC-FILE', 'CC-LANG', 'CC-FULL', 'CC-DEEP', 'CC-SOV'],
    validateonly: ['CC-FILE', 'CC-LANG', 'CC-FULL', 'CC-DEEP', 'CC-SOV']
  }
};

// File extensions considered as code files
const CODE_EXTENSIONS = ['.rs', '.js', '.ts', '.cpp', '.hpp', '.aln', '.md'];

// ============================================================================
// COMMAND PARSER
// ============================================================================

/**
 * Parse a single command line into structured format
 * @param {string} line - Command line (e.g., "TASK_B coreengine/src [Vmin=8]")
 * @returns {{opcode: string, arg: string, options: Object}|null}
 */
export function parseCommand(line) {
  // Skip comments and blank lines
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith('#')) {
    return null;
  }

  // Extract opcode (first token)
  const tokens = trimmed.split(/\s+/);
  const opcode = tokens[0];
  
  // Validate opcode
  if (!opcode.startsWith('TASK_') && opcode !== 'VERSION') {
    console.warn(`Unknown command: ${opcode}`);
    return null;
  }

  // Handle VERSION directive
  if (opcode === 'VERSION') {
    return { opcode, version: tokens[1] || '1' };
  }

  // Parse remaining tokens for arguments and options
  let arg = null;
  const options = {};

  for (let i = 1; i < tokens.length; i++) {
    const token = tokens[i];
    
    // Check for [key=value] options
    const optionMatch = token.match(/^\[(\w+)=([^\]]+)\]$/);
    if (optionMatch) {
      const [, key, value] = optionMatch;
      options[key] = value;
    } else if (!arg) {
      // First non-option token is the primary argument
      arg = token;
    }
  }

  return { opcode, arg, options };
}

/**
 * Parse a complete command block (multiple lines)
 * @param {string} commandText - Multi-line command text
 * @returns {Array} Array of parsed commands
 */
export function parseCommandBlock(commandText) {
  const lines = commandText.split('\n');
  const commands = [];
  
  for (const line of lines) {
    const parsed = parseCommand(line);
    if (parsed) {
      commands.push(parsed);
    }
  }
  
  return commands;
}

// ============================================================================
// TASK BUILDERS (Tasks A–E)
// ============================================================================

/**
 * Build Task A: Normalize paths and inject FILE headers
 * @param {string} path - Root path to process
 * @param {Object} options - Command options
 * @returns {Promise<Array>} Array of Task objects
 */
export async function buildTaskA(path, options) {
  const tasks = [];
  const tags = TAG_PRESETS.TASK_A;
  
  // List all files under path (would use Navigator/DeepWalker in real impl)
  const files = await listCodeFiles(path);
  
  for (const file of files) {
    // Validate-only task first
    tasks.push({
      kind: 'validateonly',
      path: file.path,
      content: file.content,
      sha: file.sha,
      tags: tags.validateonly
    });
    
    // Writefile task with corrected content (AI would generate this)
    // For now, placeholder - actual implementation would inject FILE header
    tasks.push({
      kind: 'writefile',
      path: file.path,
      content: file.content, // TODO: Inject FILE header and normalize path
      sha: file.sha,
      tags: tags.writefile
    });
  }
  
  return tasks;
}

/**
 * Build Task B: Enforce CC-FULL and CC-VOL
 * @param {string} path - Root path to process
 * @param {Object} options - Command options (Vmin)
 * @returns {Promise<Array>} Array of Task objects
 */
export async function buildTaskB(path, options) {
  const tasks = [];
  const tags = TAG_PRESETS.TASK_B;
  const vmin = parseInt(options.Vmin, 10) || 4; // Default from policy
  
  const files = await listCodeFiles(path);
  
  for (const file of files) {
    tasks.push({
      kind: 'validateonly',
      path: file.path,
      content: file.content,
      sha: file.sha,
      tags: tags.validateonly,
      meta: { Vmin: vmin } // Hint for validator
    });
    
    // Optional: If under-volume, AI could generate writefile tasks
    // to add functions/structs (not implemented here)
  }
  
  return tasks;
}

/**
 * Build Task C: Wiring graph audit
 * @param {string} path - Path to audit (typically coreengine/src)
 * @param {Object} options - Command options
 * @returns {Promise<Array>} Array of Task objects
 */
export async function buildTaskC(path, options) {
  const tasks = [];
  const tags = TAG_PRESETS.TASK_C;
  
  // Validate wiring spec
  tasks.push({
    kind: 'validateonly',
    path: 'specs/wiring-spec.aln',
    content: '', // Would fetch from VFS
    sha: '',
    tags: tags.validateonly
  });
  
  // Generate and validate wiring graph JSON
  const wiringGraph = await generateWiringGraph(path);
  tasks.push({
    kind: 'validateonly',
    path: '.cc-cache/validation-reports/wiring-graph.json',
    content: JSON.stringify(wiringGraph, null, 2),
    sha: '',
    tags: tags.validateonly
  });
  
  return tasks;
}

/**
 * Build Task D: Blacklist scan and contamination report
 * @param {string} path - Path to scan
 * @param {Object} options - Command options
 * @returns {Promise<Array>} Array of Task objects
 */
export async function buildTaskD(path, options) {
  const tasks = [];
  const tags = TAG_PRESETS.TASK_D;
  
  const files = await listCodeFiles(path);
  
  for (const file of files) {
    tasks.push({
      kind: 'validateonly',
      path: file.path,
      content: file.content,
      sha: file.sha,
      tags: tags.validateonly,
      scanProfile: {
        blacklistEnabled: true,
        blacklistTerms: ['Rust Syn', 'Tree-Sitter', '... omitted ...', 'rest of code']
      }
    });
  }
  
  return tasks;
}

/**
 * Build Task E: Connector adapter scaffolding
 * @param {string} platform - Platform name (e.g., "github")
 * @param {Object} options - Command options
 * @returns {Promise<Array>} Array of Task objects
 */
export async function buildTaskE(platform, options) {
  const tasks = [];
  const tags = TAG_PRESETS.TASK_E;
  const adapterPath = `connectors/${platform}/adapter.js`;
  
  // Check if adapter exists (would use VFS in real impl)
  const adapterExists = await checkFileExists(adapterPath);
  
  if (!adapterExists) {
    // Create scaffold
    const scaffoldContent = generateAdapterScaffold(platform);
    tasks.push({
      kind: 'writefile',
      path: adapterPath,
      content: scaffoldContent,
      sha: '',
      tags: tags.writefile
    });
  }
  
  // Always validate
  tasks.push({
    kind: 'validateonly',
    path: adapterPath,
    content: '', // Would fetch current content
    sha: '',
    tags: tags.validateonly
  });
  
  return tasks;
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * List all code files under a path
 * @param {string} rootPath - Root directory path
 * @returns {Promise<Array>} Array of {path, content, sha}
 */
async function listCodeFiles(rootPath) {
  // In real implementation, use Navigator/DeepWalker via WASM
  // For now, placeholder that would call cclistdir recursively
  console.log(`Listing code files under ${rootPath}`);
  
  // Mock implementation - would be replaced with actual VFS traversal
  return [
    { path: `${rootPath}/example.rs`, content: '// example', sha: 'abc123' }
  ];
}

/**
 * Generate wiring graph for a path
 * @param {string} path - Path to analyze
 * @returns {Promise<Object>} Wiring graph object
 */
async function generateWiringGraph(path) {
  // Would use cc-token-walker to analyze module dependencies
  return {
    root: path,
    nodes: [],
    edges: [],
    generatedAt: new Date().toISOString()
  };
}

/**
 * Check if a file exists in VFS
 * @param {string} path - File path
 * @returns {Promise<boolean>}
 */
async function checkFileExists(path) {
  // Would query VFS
  return false; // Assume doesn't exist for now
}

/**
 * Generate adapter scaffold for a platform
 * @param {string} platform - Platform name
 * @returns {string} JavaScript code for adapter
 */
function generateAdapterScaffold(platform) {
  return `/**
 * FILE .path: connectors/${platform}/adapter.js
 * CC-TAGS: CC-FILE, CC-LANG, CC-FULL, CC-DEEP, CC-SOV
 * 
 * Adapter contract for ${platform} connector
 * Exports: init, fetchRepo, sendResult
 */

export async function init(config) {
  // Initialize ${platform} connection
  console.log('Initializing ${platform} connector');
  return { connected: true };
}

export async function fetchRepo(owner, repo, branch = 'main') {
  // Fetch repository structure and files
  // Returns VFS snapshot compatible with ccinitvfs
  throw new Error('Not implemented');
}

export async function sendResult(result) {
  // Send validation results or ResearchObjects back
  console.log('Sending result:', result);
  return { success: true };
}
`;
}

// ============================================================================
// MAIN COMPILER
// ============================================================================

/**
 * Compile command block into SITQ payload
 * @param {string} commandText - Multi-line command text
 * @returns {Promise<Object>} SITQ payload ready for ccexecutetask
 */
export async function compileCommands(commandText) {
  const commands = parseCommandBlock(commandText);
  const allTasks = [];
  
  for (const cmd of commands) {
    if (cmd.opcode === 'VERSION') {
      continue; // Handled separately
    }
    
    let tasks = [];
    
    switch (cmd.opcode) {
      case 'TASK_A':
        if (!cmd.arg) {
          throw new Error('TASK_A requires a path argument');
        }
        tasks = await buildTaskA(cmd.arg, cmd.options);
        break;
        
      case 'TASK_B':
        if (!cmd.arg) {
          throw new Error('TASK_B requires a path argument');
        }
        tasks = await buildTaskB(cmd.arg, cmd.options);
        break;
        
      case 'TASK_C':
        if (!cmd.arg) {
          throw new Error('TASK_C requires a path argument');
        }
        tasks = await buildTaskC(cmd.arg, cmd.options);
        break;
        
      case 'TASK_D':
        if (!cmd.arg) {
          throw new Error('TASK_D requires a path argument');
        }
        tasks = await buildTaskD(cmd.arg, cmd.options);
        break;
        
      case 'TASK_E':
        if (!cmd.arg) {
          throw new Error('TASK_E requires a platform argument');
        }
        tasks = await buildTaskE(cmd.arg, cmd.options);
        break;
        
      default:
        console.warn(`Unknown opcode: ${cmd.opcode}`);
    }
    
    allTasks.push(...tasks);
  }
  
  // Build SITQ envelope
  const sitqPayload = {
    version: '1.0',
    profile: 'github',
    tasks: allTasks
  };
  
  return sitqPayload;
}

/**
 * Execute command block (compile + run)
 * @param {string} commandText - Multi-line command text
 * @returns {Promise<Object>} TaskReport from engine
 */
export async function executeCommands(commandText) {
  const payload = await compileCommands(commandText);
  
  // Emit ResearchObject before execution
  await emitResearchObject('pre-execution', {
    commandText,
    sitqPayload: payload,
    timestamp: new Date().toISOString()
  });
  
  // Execute via WASM
  const result = await ccexecutetask(JSON.stringify(payload));
  
  // Emit ResearchObject after execution
  await emitResearchObject('post-execution', {
    commandText,
    sitqPayload: payload,
    taskReport: JSON.parse(result),
    timestamp: new Date().toISOString()
  });
  
  return result;
}

// ============================================================================
// RESEARCH OBJECT EMITTER
// ============================================================================

/**
 * Emit ResearchObject to .cc-cache
 * @param {string} phase - 'pre-execution' or 'post-execution'
 * @param {Object} data - ResearchObject data
 */
async function emitResearchObject(phase, data) {
  const researchObject = {
    id: `ro-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    phase,
    ...data
  };
  
  const filename = `.cc-cache/validation-reports/research-object-${researchObject.id}.json`;
  
  // In browser, would use VFS write; in Node, would use fs
  // For now, log to console
  console.log('ResearchObject:', JSON.stringify(researchObject, null, 2));
  
  // TODO: Actually write to VFS via TaskQueue
  // This would be a writefile task appended to the current batch
  
  return researchObject;
}

// ============================================================================
// EXPORTS
// ============================================================================

export {
  TAG_PRESETS,
  parseCommand,
  parseCommandBlock,
  buildTaskA,
  buildTaskB,
  buildTaskC,
  buildTaskD,
  buildTaskE,
  compileCommands,
  executeCommands,
  emitResearchObject
};
