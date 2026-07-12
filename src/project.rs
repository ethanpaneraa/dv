use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::diffmodel::{self, FileDiff};
use crate::git;

pub struct Project {
    pub name: String,
    pub root: PathBuf,
    pub files: Vec<FileDiff>,
}

/// Finds immediate subdirectories of `scan_dir` that are git repositories.
pub fn discover(scan_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    for entry in std::fs::read_dir(scan_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && git::is_repo(&path) {
            repos.push(path);
        }
    }
    repos.sort();
    Ok(repos)
}

/// Loads a project's diff. Returns `None` if `root` isn't a git repo, or if it has no
/// changes to show.
pub fn load(root: &Path, staged: bool) -> Result<Option<Project>> {
    if !git::is_repo(root) {
        eprintln!("skipping {}: not a git repository", root.display());
        return Ok(None);
    }

    let diff_text = git::diff_in(root, staged)?;
    let files = diffmodel::parse(&diff_text);
    if files.is_empty() {
        return Ok(None);
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    Ok(Some(Project {
        name,
        root: root.to_path_buf(),
        files,
    }))
}
