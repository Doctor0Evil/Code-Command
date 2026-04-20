// FILE: ./core/engine/src/task_queue.rs

use std::collections::HashMap;

use crate::path::PathCanonicalizer;
use crate::validator::{run_validation, ValidationRequest, ValidationResult};
use crate::vfs::{TransactionalVfs, Vfs};

/// Represents a single high-level task in the Single-Iteration Task Queue.
#[derive(Clone, Debug)]
pub enum TaskKind {
    WriteFile,
    DeleteFile,
    ValidateOnly,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub kind: TaskKind,
    pub path: String,
    pub content: String,
    pub sha: String,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub enum PersistenceBackend {
    Github,
    Local,
    MemoryOnly,
}

#[derive(Debug)]
pub struct PersistOperation {
    pub kind: String,
    pub path: String,
    pub content: Option<String>,
    pub sha: String,
}

/// Execution report for a queue run.
#[derive(Clone, Debug)]
pub struct TaskReport {
    pub ok: bool,
    pub operations: Vec<String>,
    pub validations: HashMap<String, ValidationResult>,
    pub persist_changes: Vec<PersistOperation>,
    pub error: Option<String>,
}

impl TaskReport {
    pub fn new() -> Self {
        Self {
            ok: true,
            operations: Vec::new(),
            validations: HashMap::new(),
            persist_changes: Vec::new(),
            error: None,
        }
    }

    pub fn add_op(&mut self, msg: String) {
        self.operations.push(msg);
    }

    pub fn add_validation(&mut self, path: String, result: ValidationResult) {
        if !result.ok {
            self.ok = false;
        }
        self.validations.insert(path, result);
    }

    pub fn to_json(&self) -> String {
        // {"ok":true,"operations":[...],"validations":{"path":{...}}}
        let mut out = String::new();
        out.push_str("{\"ok\":");
        out.push_str(if self.ok { "true" } else { "false" });
        out.push_str(",\"operations\":[");
        for (i, op) in self.operations.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(op));
            out.push('"');
        }
        out.push("],\"validations\":{");

        let mut first = true;
        for (path, res) in &self.validations {
            if !first {
                out.push(',');
            }
            first = false;
            out.push('"');
            out.push_str(&escape_json(path));
            out.push_str("\":");
            out.push_str(&res.to_json());
        }

        out.push('}');

        // Add persist_changes array
        out.push_str(",\"persist_changes\":[");
        for (i, pc) in self.persist_changes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"kind\":\"");
            out.push_str(&escape_json(&pc.kind));
            out.push_str("\",\"path\":\"");
            out.push_str(&escape_json(&pc.path));
            out.push_str("\",\"sha\":\"");
            out.push_str(&escape_json(&pc.sha));
            out.push_str("\",\"content\":");
            match &pc.content {
                Some(c) => {
                    out.push('"');
                    out.push_str(&escape_json(c));
                    out.push('"');
                }
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push(']');

        // Add error field
        out.push_str(",\"error\":");
        match &self.error {
            Some(e) => {
                out.push('"');
                out.push_str(&escape_json(e));
                out.push('"');
            }
            None => out.push_str("null"),
        }

        out.push('}');
        out
    }
}

#[derive(Debug)]
pub struct TaskQueuePayload {
    pub profile: String,
    pub tasks: Vec<Task>,
}

/// Single-Iteration Task Queue (SITQ).
pub struct TaskQueue<'a> {
    pub vfs: &'a mut Vfs,
}

impl<'a> TaskQueue<'a> {
    pub fn new(vfs: &'a mut Vfs) -> Self {
        TaskQueue { vfs }
    }

    /// Parses a JSON-like task description into a queue.
    /// Expected very small schema, e.g.:
    /// {
    ///   "tasks":[
    ///     {"kind":"create","path":"src/main.rs","content":"...","sha":"","tags":["CC-FILE","CC-FULL"]},
    ///     {"kind":"validate","path":"src/main.rs","tags":["CC-FILE","CC-LANG","CC-FULL"]}
    ///   ]
    /// }
    /// Parsing is intentionally minimal and assumes well-formed producer.
    pub fn from_json(json: &str) -> Self {
        let mut tasks = Vec::new();

        // Find "tasks":[ ... ] region.
        if let Some(start) = json.find("\"tasks\"") {
            if let Some(open_bracket) = json[start..].find('[') {
                let start_idx = start + open_bracket + 1;
                if let Some(end_bracket_rel) = json[start_idx..].rfind(']') {
                    let end_idx = start_idx + end_bracket_rel;
                    let inner = &json[start_idx..end_idx];
                    // Split crude objects by "},{"; this is acceptable given our producer.
                    for raw in inner.split("},{") {
                        if let Some(task) = parse_task_object(raw) {
                            tasks.push(task);
                        }
                    }
                }
            }
        }

        Self { tasks }
    }

    /// Executes the queue against the provided VFS with nested transaction support.
    /// Reads `profile` field to determine persistence backend:
    /// - "github" -> PersistenceBackend::Github
    /// - "local" -> PersistenceBackend::Local
    /// - "memory-only" -> PersistenceBackend::MemoryOnly
    /// Unknown profiles cause error CCENG-030.
    /// After successful execution, if profile is not MemoryOnly, includes a
    /// `persist_changes` array in TaskReport with file operations.
    pub fn execute(&mut self, payload: TaskQueuePayload) -> TaskReport {
        let backend = match payload.profile.as_str() {
            "github" => PersistenceBackend::Github,
            "local" => PersistenceBackend::Local,
            "memory-only" => PersistenceBackend::MemoryOnly,
            other => {
                return TaskReport {
                    ok: false,
                    operations: Vec::new(),
                    validations: HashMap::new(),
                    persist_changes: Vec::new(),
                    error: Some(format!("CCENG-030 Unknown profile `{}`", other)),
                };
            }
        };

        let mut tx_vfs = TransactionalVfs::new(self.vfs);
        let mut validations: HashMap<String, ValidationResult> = HashMap::new();
        let mut persist_changes: Vec<PersistOperation> = Vec::new();
        let mut operations: Vec<String> = Vec::new();

        // Top-level transaction
        tx_vfs.begin_tx();

        for task in &payload.tasks {
            // Nesting: allow composite tasks to open inner transactions
            tx_vfs.begin_tx();

            let canonicalizer = PathCanonicalizer::new();
            let Some(norm_path) = canonicalizer.canonicalize(&task.path) else {
                tx_vfs.rollback_tx();
                return TaskReport {
                    ok: false,
                    operations,
                    validations,
                    persist_changes,
                    error: Some(format!("Invalid path `{}`", task.path)),
                };
            };

            // Validation phase for write/delete/validate-only
            let code = task.content.as_str();
            let tags_json = tags_to_json_array(&task.tags);
            let req = ValidationRequest {
                code: code.to_string(),
                tags: task.tags.clone(),
                previous_symbols: Vec::new(),
            };
            let validation = run_validation(req);
            let ok = validation.ok;
            validations.insert(norm_path.clone(), validation);

            if !ok {
                // rollback inner and outer transaction, fail entire payload
                tx_vfs.rollback_tx();
                tx_vfs.rollback_tx();
                return TaskReport {
                    ok: false,
                    operations,
                    validations,
                    persist_changes,
                    error: Some("Task failed validation; transaction rolled back".to_string()),
                };
            }

            // Apply operation into transactional VFS
            match task.kind {
                TaskKind::WriteFile => {
                    if tx_vfs.write(&norm_path, &task.content, &task.sha).is_err() {
                        tx_vfs.rollback_tx();
                        tx_vfs.rollback_tx();
                        return TaskReport {
                            ok: false,
                            operations,
                            validations,
                            persist_changes,
                            error: Some("Failed to write file in VFS".to_string()),
                        };
                    }
                    operations.push(format!("write: {}", norm_path));

                    // Record logical persist operation
                    if !matches!(backend, PersistenceBackend::MemoryOnly) {
                        persist_changes.push(PersistOperation {
                            kind: "write".to_string(),
                            path: norm_path.clone(),
                            content: Some(task.content.clone()),
                            sha: task.sha.clone(),
                        });
                    }
                }
                TaskKind::DeleteFile => {
                    if tx_vfs.delete(&norm_path).is_err() {
                        tx_vfs.rollback_tx();
                        tx_vfs.rollback_tx();
                        return TaskReport {
                            ok: false,
                            operations,
                            validations,
                            persist_changes,
                            error: Some("Failed to delete file in VFS".to_string()),
                        };
                    }
                    operations.push(format!("delete: {}", norm_path));

                    if !matches!(backend, PersistenceBackend::MemoryOnly) {
                        persist_changes.push(PersistOperation {
                            kind: "delete".to_string(),
                            path: norm_path.clone(),
                            content: None,
                            sha: task.sha.clone(),
                        });
                    }
                }
                TaskKind::ValidateOnly => {
                    operations.push(format!("validate: {}", norm_path));
                    // no VFS mutation, but nested tx ensures any future writes
                    // inside composite flows are isolated until commit
                }
            }

            // Inner tx success
            tx_vfs.commit_tx();
        }

        // Top-level success
        tx_vfs.commit_tx();

        TaskReport {
            ok: true,
            operations,
            validations,
            persist_changes: if matches!(backend, PersistenceBackend::MemoryOnly) {
                Vec::new()
            } else {
                persist_changes
            },
            error: None,
        }
    }

    pub fn empty_failure(reason: &str) -> TaskReport {
        let mut report = TaskReport::new();
        report.ok = false;
        report.add_op(format!("queue-failed: {}", reason));
        report
    }
}

/* ---------- Parsing Helpers (Custom, No External JSON) ---------- */ [file:2]

fn parse_task_object(raw: &str) -> Option<Task> {
    let mut kind = String::new();
    let mut path = String::new();
    let mut content = String::new();
    let mut sha = String::new();
    let mut tags: Vec<String> = Vec::new();

    for part in raw.split(',') {
        let p = part.trim();
        if p.starts_with("\"kind\"") {
            if let Some(v) = extract_json_value(p) {
                kind = v;
            }
        } else if p.starts_with("\"path\"") {
            if let Some(v) = extract_json_value(p) {
                path = v;
            }
        } else if p.starts_with("\"content\"") {
            if let Some(v) = extract_json_value(p) {
                content = v;
            }
        } else if p.starts_with("\"sha\"") {
            if let Some(v) = extract_json_value(p) {
                sha = v;
            }
        } else if p.starts_with("\"tags\"") {
            // Expect "tags":["TAG","TAG2",...]. [file:2]
            if let Some(open) = p.find('[') {
                let tags_inner = &p[open + 1..];
                let tags_clean = tags_inner.trim_end_matches(']').trim_end_matches('}');
                for t_raw in tags_clean.split(',') {
                    let trimmed = t_raw.trim().trim_matches('"');
                    if !trimmed.is_empty() {
                        tags.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    if path.is_empty() {
        return None;
    }

    let kind_enum = match kind.as_str() {
        "create" => TaskKind::CreateFile,
        "update" => TaskKind::UpdateFile,
        "validate" => TaskKind::ValidateFile,
        _ => TaskKind::CreateFile,
    };

    Some(Task {
        kind: kind_enum,
        path,
        content,
        sha,
        tags,
    })
}

/* ---------- Small Utilities ---------- */ [file:2]

fn extract_json_value(part: &str) -> Option<String> {
    let mut split = part.splitn(2, ':');
    split.next()?;
    let value_part = split.next()?.trim();
    let value_trimmed = value_part
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_end_matches('}')
        .trim_end_matches(']');
    Some(value_trimmed.to_string())
}

fn escape_json(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn tags_to_json_array(tags: &[String]) -> String {
    let mut out = String::new();
    out.push('[');
    for (i, t) in tags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape_json(t));
        out.push('"');
    }
    out.push(']');
    out
}
