use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("path is outside the workspace")]
    OutsideWorkspace,
    #[error("file is too large to edit safely")]
    TooLarge,
    #[error("workspace operation failed: {0}")]
    Io(#[from] std::io::Error),
}

const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

pub fn list_text_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_type().is_dir() || !should_skip_directory(entry.path(), root)
        })
        .filter_map(Result::ok)
    {
        if files.len() >= 500 {
            break;
        }
        let path = entry.path();
        if entry.file_type().is_file() && is_text_like(path) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

pub fn read_text(root: &Path, path: &Path) -> Result<String, WorkspaceError> {
    let safe = resolve_existing(root, path)?;
    let metadata = fs::metadata(&safe)?;
    if metadata.len() > MAX_TEXT_BYTES {
        return Err(WorkspaceError::TooLarge);
    }
    Ok(fs::read_to_string(safe).unwrap_or_else(|_| String::from("[Binary or non-UTF-8 file]")))
}

pub fn write_text(root: &Path, path: &Path, contents: &str) -> Result<(), WorkspaceError> {
    let safe = resolve_for_write(root, path)?;
    if contents.len() as u64 > MAX_TEXT_BYTES {
        return Err(WorkspaceError::TooLarge);
    }
    fs::write(safe, contents)?;
    Ok(())
}

fn resolve_existing(root: &Path, path: &Path) -> Result<PathBuf, WorkspaceError> {
    let canonical_root = root.canonicalize()?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = candidate.canonicalize()?;
    if canonical.starts_with(&canonical_root) {
        Ok(canonical)
    } else {
        Err(WorkspaceError::OutsideWorkspace)
    }
}

fn resolve_for_write(root: &Path, path: &Path) -> Result<PathBuf, WorkspaceError> {
    let canonical_root = root.canonicalize()?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let parent = candidate
        .parent()
        .ok_or(WorkspaceError::OutsideWorkspace)?
        .canonicalize()?;
    if !parent.starts_with(&canonical_root) {
        return Err(WorkspaceError::OutsideWorkspace);
    }
    if candidate.exists() {
        let canonical = candidate.canonicalize()?;
        if !canonical.starts_with(&canonical_root) {
            return Err(WorkspaceError::OutsideWorkspace);
        }
        return Ok(canonical);
    }
    let name = candidate
        .file_name()
        .ok_or(WorkspaceError::OutsideWorkspace)?;
    Ok(parent.join(name))
}

fn should_skip_directory(path: &Path, root: &Path) -> bool {
    if path == root {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target" | "node_modules" | "sources"))
}

fn is_text_like(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "rs" | "toml"
                    | "json"
                    | "md"
                    | "txt"
                    | "js"
                    | "ts"
                    | "tsx"
                    | "jsx"
                    | "css"
                    | "html"
                    | "yml"
                    | "yaml"
                    | "sh"
                    | "ps1"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_outside_path() {
        let root = std::env::temp_dir().join(format!("ns-plasma-tools-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        let result = write_text(&root, Path::new("..\\outside.txt"), "nope");
        assert!(matches!(result, Err(WorkspaceError::OutsideWorkspace)));
        let _ = fs::remove_dir_all(root);
    }
}
