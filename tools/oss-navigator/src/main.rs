// FILE ./tools/oss-navigator/src/main.rs
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// Default root of the OSSFS mount.
const DEFAULT_OSS_ROOT: &str = "/mnt/oss";

/// Default target directory root (shared Cargo target dir).
const DEFAULT_TARGET_ROOT: &str = "/mnt/oss/target";

/// Simple logger with a fixed prefix.
fn log(msg: &str) {
    eprintln!("[oss-navigator] {msg}");
}

/// Aggregate statistics for a crate directory.
#[derive(Clone, Debug, Default)]
struct CrateUsage {
    bytes: u64,
    files: u64,
}

/// Aggregate statistics for a target profile directory (debug, release, incremental).
#[derive(Clone, Debug, Default)]
struct ProfileUsage {
    bytes: u64,
    files: u64,
}

/// Walk a directory tree recursively using only std::fs, counting total size and file count.
///
/// This is the core CC-NAV custom navigation logic: a manual recursion over PathBuf entries,
/// without any external traversal libraries.[file:3]
fn walk_dir_size(root: &Path) -> io::Result<(u64, u64)> {
    let mut total_bytes = 0u64;
    let mut total_files = 0u64;
    let mut stack: Vec<PathBuf> = Vec::new();

    stack.push(root.to_path_buf());

    while let Some(path) = stack.pop() {
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(err) => {
                log(&format!(
                    "Warning: cannot read metadata for '{}': {err}",
                    path.display()
                ));
                continue;
            }
        };

        if meta.is_dir() {
            let read_dir = match fs::read_dir(&path) {
                Ok(rd) => rd,
                Err(err) => {
                    log(&format!(
                        "Warning: cannot read directory '{}': {err}",
                        path.display()
                    ));
                    continue;
                }
            };

            for entry in read_dir {
                match entry {
                    Ok(e) => stack.push(e.path()),
                    Err(err) => {
                        log(&format!("Warning: failed to read entry in '{}': {err}", path.display()));
                    }
                }
            }
        } else if meta.is_file() {
            total_files = total_files.saturating_add(1);
            total_bytes = total_bytes.saturating_add(meta.len());
        } else {
            // symlink or special; skip for simplicity
        }
    }

    Ok((total_bytes, total_files))
}

/// Detects whether a directory looks like a Rust crate by checking for Cargo.toml.
fn is_rust_crate_dir(path: &Path) -> bool {
    let manifest = path.join("Cargo.toml");
    manifest.is_file()
}

/// Scan the OSS root for Rust crates and compute their aggregate size.
///
/// Strategy:
/// - Walk only directories directly under /mnt/oss (and optionally /mnt/oss/projects like patterns if desired).
/// - For each directory containing Cargo.toml, recurse and compute total size.
fn collect_crate_usage(oss_root: &Path) -> io::Result<BTreeMap<String, CrateUsage>> {
    let mut crates: BTreeMap<String, CrateUsage> = BTreeMap::new();

    let read_dir = match fs::read_dir(oss_root) {
        Ok(rd) => rd,
        Err(err) => {
            log(&format!(
                "ERROR: cannot read OSS root '{}': {err}",
                oss_root.display()
            ));
            return Err(err);
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log(&format!("Warning: failed to read entry in '{}': {err}", oss_root.display()));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if is_rust_crate_dir(&path) {
            let crate_name = path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>")
                .to_string();
            log(&format!("Found Rust crate '{}'", crate_name));

            match walk_dir_size(&path) {
                Ok((bytes, files)) => {
                    crates.insert(
                        crate_name,
                        CrateUsage {
                            bytes,
                            files,
                        },
                    );
                }
                Err(err) => {
                    log(&format!(
                        "Warning: failed to compute size for crate '{}': {err}",
                        path.display()
                    ));
                }
            }
        }
    }

    Ok(crates)
}

/// Collect usage for target profiles (debug, release, incremental) under the shared target root.
fn collect_target_profile_usage(target_root: &Path) -> io::Result<BTreeMap<String, ProfileUsage>> {
    let mut profiles: BTreeMap<String, ProfileUsage> = BTreeMap::new();

    // For each of these known profile directories, if present, compute size.
    for name in &["debug", "release", "incremental"] {
        let path = target_root.join(name);
        if !path.is_dir() {
            continue;
        }

        log(&format!(
            "Scanning target profile '{}' under '{}'",
            name,
            target_root.display()
        ));

        match walk_dir_size(&path) {
            Ok((bytes, files)) => {
                profiles.insert(
                    name.to_string(),
                    ProfileUsage {
                        bytes,
                        files,
                    },
                );
            }
            Err(err) => {
                log(&format!(
                    "Warning: failed to compute size for profile '{}': {err}",
                    path.display()
                ));
            }
        }
    }

    Ok(profiles)
}

/// Pretty-print bytes in a human-friendly unit (KiB, MiB, GiB, TiB).
fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    const TIB: u64 = 1024 * 1024 * 1024 * 1024;

    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Ensure OSS root exists and is readable.
fn ensure_dir(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path '{}' does not exist", path.display()),
        ));
    }
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Path '{}' is not a directory", path.display()),
        ));
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let oss_root = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(DEFAULT_OSS_ROOT)
    };

    let target_root = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from(DEFAULT_TARGET_ROOT)
    };

    if let Err(err) = ensure_dir(&oss_root) {
        log(&format!(
            "ERROR: invalid OSS root '{}': {err}",
            oss_root.display()
        ));
        std::process::exit(1);
    }

    log(&format!("Using OSS root: '{}'", oss_root.display()));
    log(&format!("Using target root: '{}'", target_root.display()));

    // --- Crate usage --------------------------------------------------------

    match collect_crate_usage(&oss_root) {
        Ok(crate_usage) => {
            println!("== Crate usage under {} ==", oss_root.display());
            if crate_usage.is_empty() {
                println!("(no Rust crates with Cargo.toml found)");
            } else {
                for (name, usage) in crate_usage {
                    println!(
                        "- crate: {:30} size: {:>12}  files: {}",
                        name,
                        format_bytes(usage.bytes),
                        usage.files
                    );
                }
            }
        }
        Err(err) => {
            log(&format!(
                "ERROR: failed to collect crate usage under '{}': {err}",
                oss_root.display()
            ));
        }
    }

    // --- Target profile usage ----------------------------------------------

    if target_root.is_dir() {
        match collect_target_profile_usage(&target_root) {
            Ok(profile_usage) => {
                println!();
                println!(
                    "== Target profile usage under {} ==",
                    target_root.display()
                );
                if profile_usage.is_empty() {
                    println!("(no debug/release/incremental profiles found)");
                } else {
                    for (name, usage) in profile_usage {
                        println!(
                            "- profile: {:11} size: {:>12}  files: {}",
                            name,
                            format_bytes(usage.bytes),
                            usage.files
                        );
                    }
                }
            }
            Err(err) => {
                log(&format!(
                    "ERROR: failed to collect target profile usage under '{}': {err}",
                    target_root.display()
                ));
            }
        }
    } else {
        log(&format!(
            "Target root '{}' does not exist or is not a directory; skipping profile report.",
            target_root.display()
        ));
    }

    // Optional: touch a marker to document that this navigator has been run.
    let marker = oss_root.join(".cc_oss_navigator_last_run");
    if let Err(err) = File::create(&marker) {
        log(&format!(
            "Warning: failed to write navigator marker '{}': {err}",
            marker.display()
        ));
    }
}
