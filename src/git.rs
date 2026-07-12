use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

pub fn diff_in(dir: &Path, staged: bool) -> Result<String> {
    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--staged");
    }

    let output = Command::new("git").args(&args).current_dir(dir).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed in {}: {stderr}", dir.display());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}
