// FILE: ./core/engine/src/task_queue.rs

use std::collections::HashMap; // Standard library only; no external crates. [file:2]

use crate::vfs::Vfs;
use crate::validator::{run_validation, ValidationRequest, ValidationResult};

/// Represents a single high-level task in the Single-Iteration Task Queue. [file:2]
#[derive(Clone, Debug)]
pub enum TaskKind {
    CreateFile,
    UpdateFile,
    ValidateFile,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub kind: TaskKind,
    pub path: String,
    pub content: String,
    pub sha: String,
    pub tags: Vec<String>,
}

/// Execution report for a queue run. [file:2]
#[derive(Clone, Debug)]
pub struct TaskReport {
    pub ok: bool,
    pub operations: Vec<String>,
    pub validations: HashMap<String, ValidationResult>,
}

impl TaskReport {
    pub fn new() -> Self {
        Self {
            ok: true,
            operations: Vec::new(),
            validations: HashMap::new(),
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
        // {"ok":true,"operations":[...],"validations":{"path":{...}}} [file:2]
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
        out.push_str("],\"validations\":{");

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

        out.push_str("}}");
        out
    }
}

/// Single-Iteration Task Queue (SITQ). [file:2]
pub struct TaskQueue {
    pub tasks: Vec<Task>,
}

impl TaskQueue {
    /// Parses a JSON-like task description into a queue.
    /// Expected very small schema, e.g.:
    /// {
    ///   "tasks":[
    ///     {"kind":"create","path":"src/main.rs","content":"...","sha":"","tags":["CC-FILE","CC-FULL"]},
    ///     {"kind":"validate","path":"src/main.rs","tags":["CC-FILE","CC-LANG","CC-FULL"]}
    ///   ]
    /// }
    /// Parsing is intentionally minimal and assumes well-formed producer. [file:2]
    pub fn from_json(json: &str) -> Self {
        let mut tasks = Vec::new();

        // Find "tasks":[ ... ] region. [file:2]
        if let Some(start) = json.find("\"tasks\"") {
            if let Some(open_bracket) = json[start..].find('[') {
                let start_idx = start + open_bracket + 1;
                if let Some(end_bracket_rel) = json[start_idx..].rfind(']') {
                    let end_idx = start_idx + end_bracket_rel;
                    let inner = &json[start_idx..end_idx];
                    // Split crude objects by "},{"; this is acceptable given our producer. [file:2]
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

    /// Executes the queue against the provided VFS, enforcing CC-VOL by nature
    /// (multiple tasks/actions in one call). [file:2]
    pub fn execute(&mut self, vfs: &mut Vfs) -> TaskReport {
        let mut report = TaskReport::new();

        for task in &self.tasks {
            match task.kind {
                TaskKind::CreateFile | TaskKind::UpdateFile => {
                    let ok = vfs.write(&task.path, &task.content, &task.sha);
                    let label = match task.kind {
                        TaskKind::CreateFile => "create",
                        TaskKind::UpdateFile => "update",
                        _ => "write",
                    };
                    if ok {
                        report.add_op(format!("{}: {}", label, task.path));
                    } else {
                        report.ok = false;
                        report.add_op(format!("{}-failed: {}", label, task.path));
                    }
                }
                TaskKind::ValidateFile => {
                    let content = vfs.read(&task.path).unwrap_or_default();
                    let tags_json = tags_to_json_array(&task.tags);
                    let req = ValidationRequest {
                        code: content,
                        tags: task.tags.clone(),
                        previous_symbols: Vec::new(),
                    };
                    let res = run_validation(req);
                    report.add_validation(task.path.clone(), res);
                }
            }
        }

        report
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
