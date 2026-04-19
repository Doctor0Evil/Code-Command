// FILE ./tools/clean-oss-target/src/main.rs
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DEFAULT_TARGET_ROOT: &str = "/mnt/oss/target";
const DAYS_7: u64 = 7;
const DAYS_30: u64 = 30;

/// Simple logger with a fixed prefix, so output is easy to scan.
fn log(msg: &str) {
    eprintln!("[clean-oss-target] {msg}");
}

fn main() {
    let target_root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGET_ROOT));

    if !target_root.exists() {
        log(&format!(
            "Target root '{}' does not exist; nothing to clean.",
            target_root.display()
        ));
        return;
    }

    if !target_root.is_dir() {
        log(&format!(
            "Target root '{}' is not a directory; aborting.",
            target_root.display()
        ));
        std::process::exit(1);
    }

    let now = SystemTime::now();
    let seven_days = Duration::from_secs(DAYS_7 * 24 * 60 * 60);
    let thirty_days = Duration::from_secs(DAYS_30 * 24 * 60 * 60);

    if let Err(err) = clean_target_root(&target_root, now, seven_days, thirty_days) {
        log(&format!("ERROR during cleanup: {err}"));
        std::process::exit(1);
    }
}

/// Entry point for cleaning logic; separated for testability.
fn clean_target_root(
    root: &Path,
    now: SystemTime,
    incremental_age: Duration,
    build_age: Duration,
) -> io::Result<()> {
    log(&format!("Scanning '{}' ...", root.display()));
    for entry in fs::read_dir(root)? {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log(&format!("Warning: unable to read entry: {err}"));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        match name {
            "incremental" => {
                prune_incremental_dir(&path, now, incremental_age)?;
            }
            "debug" | "release" => {
                prune_build_dir(&path, now, build_age)?;
            }
            _ => {
                // Other directories under target are ignored (e.g., target/<triple>).
            }
        }
    }

    Ok(())
}

/// Remove subdirectories under `incremental/` whose modified time is older than
/// `max_age`.
fn prune_incremental_dir(dir: &Path, now: SystemTime, max_age: Duration) -> io::Result<()> {
    log(&format!("Inspecting incremental cache at '{}'", dir.display()));

    for entry in fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log(&format!(
                    "Warning: unable to read incremental entry: {err}"
                ));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(err) => {
                log(&format!(
                    "Warning: unable to read metadata for '{}': {err}",
                    path.display()
                ));
                continue;
            }
        };

        let modified = metadata.modified().unwrap_or_else(|_| SystemTime::UNIX_EPOCH);
        if is_older_than(now, modified, max_age) {
            log(&format!(
                "Removing stale incremental directory '{}'",
                path.display()
            ));
            if let Err(err) = fs::remove_dir_all(&path) {
                log(&format!(
                    "Warning: failed to remove '{}': {err}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

/// Remove `debug/` or `release/` directory if its last access time is older than
/// `max_age`.
fn prune_build_dir(dir: &Path, now: SystemTime, max_age: Duration) -> io::Result<()> {
    log(&format!("Inspecting build dir '{}'", dir.display()));

    let metadata = match fs::metadata(dir) {
        Ok(m) => m,
        Err(err) => {
            log(&format!(
                "Warning: unable to read metadata for '{}': {err}",
                dir.display()
            ));
            return Ok(());
        }
    };

    // Prefer atime (access time) if available; fall back to mtime if not.
    let atime = metadata.accessed().or_else(|_| metadata.modified());

    let last_used = match atime {
        Ok(t) => t,
        Err(_) => {
            log(&format!(
                "Warning: unable to determine last access for '{}', skipping.",
                dir.display()
            ));
            return Ok(());
        }
    };

    if is_older_than(now, last_used, max_age) {
        log(&format!(
            "Removing stale build directory '{}'",
            dir.display()
        ));
        if let Err(err) = fs::remove_dir_all(dir) {
            log(&format!(
                "Warning: failed to remove '{}': {err}",
                dir.display()
            ));
        }
    }

    Ok(())
}

/// Returns true if `time` is strictly older than `max_age` before `now`.
fn is_older_than(now: SystemTime, time: SystemTime, max_age: Duration) -> bool {
    match now.duration_since(time) {
        Ok(elapsed) => elapsed > max_age,
        Err(_) => false, // time is in the future; treat as fresh
    }
}
