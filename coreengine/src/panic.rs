// FILE: ./coreengine/src/panic.rs
use std::panic;

pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };

        let location = info.location();
        let file = location.map(|l| l.file().to_string()).unwrap_or_default();
        let line = location.map(|l| l.line()).unwrap_or(0);

        let json = build_panic_json(&file, line, &message);
        // Forward to JS, ignoring errors.
        extern "C" {
            fn cc_panic_report(ptr: *const u8, len: usize);
        }
        unsafe {
            cc_panic_report(json.as_bytes().as_ptr(), json.len());
        }
    }));
}

fn build_panic_json(file: &str, line: u32, message: &str) -> String {
    // Minimal JSON, no external crates.
    let mut out = String::new();
    out.push('{');
    out.push_str("\"tag\":\"CC-PANIC\",");
    out.push_str("\"severity\":\"fatal\",");
    out.push_str("\"file\":\"");
    escape_json_into(file, &mut out);
    out.push_str("\",");
    out.push_str("\"line\":");
    out.push_str(&line.to_string());
    out.push_str(",\"message\":\"");
    escape_json_into(message, &mut out);
    out.push_str("\"}");
    out
}

fn escape_json_into(input: &str, out: &mut String) {
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
}
