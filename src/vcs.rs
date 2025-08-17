use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

async fn run_git(project_root: &Path, args: &[&str]) -> Result<(bool, String)> {
    let output = Command::new("git").args(args).current_dir(project_root).output().await?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok((output.status.success(), text))
}

pub async fn ensure_repo(project_root: &Path) -> Result<()> {
    let git_dir = project_root.join(".git");
    if git_dir.exists() { return Ok(()); }
    let (ok, out) = run_git(project_root, &["init"]).await?;
    if !ok { return Err(anyhow!("git init failed: {}", out)); }
    Ok(())
}

pub async fn commit_paths(project_root: &Path, paths: &[PathBuf], message: &str) -> Result<()> {
    if !paths.is_empty() {
        let output = {
            let mut c = Command::new("git");
            c.arg("add").arg("--");
            for p in paths { c.arg(p); }
            c.current_dir(project_root).output().await?
        };
        if !output.status.success() {
            let mut text = String::new();
            text.push_str(&String::from_utf8_lossy(&output.stdout));
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            return Err(anyhow!("git add failed: {}", text));
        }
    }
    let (ok, out) = run_git(project_root, &["commit", "--allow-empty", "-m", message]).await?;
    if !ok { return Err(anyhow!("git commit failed: {}", out)); }
    Ok(())
}

pub async fn reset_hard_head_minus_one(project_root: &Path) -> Result<()> {
    let (ok, out) = run_git(project_root, &["reset", "--hard", "HEAD~1"]).await?;
    if !ok { return Err(anyhow!("git reset failed: {}", out)); }
    Ok(())
}
