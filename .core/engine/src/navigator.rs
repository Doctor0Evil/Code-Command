// FILE .core/engine/src/navigator.rs

use std::path::{Path, PathBuf};
use std::fs;

pub struct VfsEntry {
    pub path: String,     // normalized, VFS-style path
    pub is_dir: bool,
    pub sha: Option<String>,
}

pub struct DeepWalker<'a> {
    root: PathBuf,
    max_depth: usize,
    exts: Option<Vec<&'a str>>,
    stack: Vec<(PathBuf, usize)>, // (path, depth)
}

impl<'a> DeepWalker<'a> {
    pub fn new(root: &Path, max_depth: usize) -> Self {
        let mut stack = Vec::new();
        stack.push((root.to_path_buf(), 0));
        DeepWalker {
            root: root.to_path_buf(),
            max_depth,
            exts: None,
            stack,
        }
    }

    pub fn filter_extensions(mut self, exts: &'a [&'a str]) -> Self {
        self.exts = Some(exts.to_vec());
        self
    }

    fn should_include(&self, path: &Path, is_dir: bool) -> bool {
        if is_dir {
            return true;
        }
        if let Some(ref exts) = self.exts {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                return exts.iter().any(|e| *e == ext);
            }
            return false;
        }
        true
    }
}

impl<'a> Iterator for DeepWalker<'a> {
    type Item = (PathBuf, VfsEntry);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((path, depth)) = self.stack.pop() {
            let meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let is_dir = meta.is_dir();

            // Push children if within depth
            if is_dir && depth < self.max_depth {
                if let Ok(read_dir) = fs::read_dir(&path) {
                    for entry in read_dir.flatten() {
                        self.stack.push((entry.path(), depth + 1));
                    }
                }
            }

            if !self.should_include(&path, is_dir) {
                continue;
            }

            let vfs_entry = VfsEntry {
                path: path.to_string_lossy().into_owned(),
                is_dir,
                sha: None,
            };
            return Some((path, vfs_entry));
        }
        None
    }
}
