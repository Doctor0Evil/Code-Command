// FILE ./tools/oss-du/src/main.rs
use std::cmp::Reverse;
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Represents a single file entry tracked as one of the "largest N" files.
#[derive(Debug, Clone)]
struct FileStat {
    size: u64,
    path: PathBuf,
}

/// Aggregated disk-usage statistics for a subtree.
#[derive(Debug, Default)]
struct UsageStats {
    total_size: u64,
    file_count: u64,
    largest: Vec<FileStat>, // maintained as min-heap semantics with manual upkeep
}

impl UsageStats {
    fn new() -> Self {
        UsageStats {
            total_size: 0,
            file_count: 0,
            largest: Vec::new(),
        }
    }

    fn record_file(&mut self, path: PathBuf, size: u64, top_n: usize) {
        self.total_size = self
            .total_size
            .saturating_add(size);
        self.file_count = self.file_count.saturating_add(1);

        let entry = FileStat { size, path };

        if self.largest.len() < top_n {
            self.largest.push(entry);
            self.largest.sort_by_key(|e| e.size);
        } else if let Some(min) = self.largest.first() {
            if size > min.size {
                // Replace smallest and keep sorted ascending.
                self.largest[0] = entry;
                self.largest.sort_by_key(|e| e.size);
            }
        }
    }

    fn merge(&mut self, other: UsageStats, top_n: usize) {
        self.total_size = self
            .total_size
            .saturating_add(other.total_size);
        self.file_count = self.file_count.saturating_add(other.file_count);

        for fs in other.largest {
            self.record_file(fs.path, fs.size, top_n);
        }
    }
}

/// Simple logger.
fn log(msg: &str) {
    eprintln!("[oss-du] {msg}");
}

/// Entry point.
fn main() {
    // Default root path if none is provided.
    let default_root = PathBuf::from("/mnt/oss");
    let mut args = env::args().skip(1);

    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or(default_root);

    let top_n = 10usize;

    if !root.exists() {
        log(&format!(
            "ERROR: root path '{}' does not exist.",
            root.display()
        ));
        std::process::exit(1);
    }

    let mut visited = Vec::<(u64, u64)>::new(); // (dev, ino) pairs to detect symlink loops.

    match scan_path(&root, &mut visited, top_n) {
        Ok(stats) => {
            println!("Root: {}", root.display());
            println!("Total files: {}", stats.file_count);
            println!("Total size (bytes): {}", stats.total_size);
            println!();
            println!("Largest {} files:", top_n);

            // Print in descending order by size.
            let mut largest = stats.largest.clone();
            largest.sort_by_key(|e| Reverse(e.size));

            for fs in largest {
                println!("{:>16}  {}", fs.size, fs.path.display());
            }
        }
        Err(err) => {
            log(&format!("ERROR while scanning '{}': {err}", root.display()));
            std::process::exit(1);
        }
    }
}

/// Recursively scan a path, aggregating usage statistics.
/// Symlinks are followed, but dev/inode pairs are tracked to avoid infinite loops
/// on cyclic links, which is important on virtual/OSS-style mounts.[file:3]
fn scan_path(
    path: &Path,
    visited: &mut Vec<(u64, u64)>,
    top_n: usize,
) -> io::Result<UsageStats> {
    let mut stats = UsageStats::new();

    // Use lstat-like behavior first to decide how to handle the entry.
    let meta = fs::symlink_metadata(path)?;

    let file_type = meta.file_type();
    if file_type.is_file() {
        let size = meta.len();
        stats.record_file(path.to_path_buf(), size, top_n);
        return Ok(stats);
    }

    if file_type.is_symlink() {
        // Follow the symlink once, but avoid loops via dev+ino tracking.[file:3]
        let target_meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                log(&format!(
                    "Warning: unable to stat symlink target '{}': {e}",
                    path.display()
                ));
                return Ok(stats);
            }
        };

        let dev = target_meta.dev();
        let ino = target_meta.ino();
        if visited.contains(&(dev, ino)) {
            log(&format!(
                "Warning: detected symlink loop at '{}'; skipping.",
                path.display()
            ));
            return Ok(stats);
        }

        visited.push((dev, ino));

        if target_meta.is_file() {
            let size = target_meta.len();
            stats.record_file(path.to_path_buf(), size, top_n);
            return Ok(stats);
        }

        if target_meta.is_dir() {
            return scan_directory(path, visited, top_n);
        }

        // Non-regular target; ignore.
        return Ok(stats);
    }

    if file_type.is_dir() {
        return scan_directory(path, visited, top_n);
    }

    // Other special file types (sockets, FIFOs, etc.) are ignored.
    Ok(stats)
}

/// Scan a directory and all its descendants.
fn scan_directory(
    dir: &Path,
    visited: &mut Vec<(u64, u64)>,
    top_n: usize,
) -> io::Result<UsageStats> {
    let mut stats = UsageStats::new();

    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(err) => {
            log(&format!(
                "Warning: unable to read directory '{}': {err}",
                dir.display()
            ));
            return Ok(stats);
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log(&format!(
                    "Warning: unable to read directory entry in '{}': {err}",
                    dir.display()
                ));
                continue;
            }
        };

        let path = entry.path();

        match scan_path(&path, visited, top_n) {
            Ok(child_stats) => {
                stats.merge(child_stats, top_n);
            }
            Err(err) => {
                log(&format!("Warning: error scanning '{}': {err}", path.display()));
            }
        }
    }

    Ok(stats)
}
