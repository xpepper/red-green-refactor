use crate::providers::{EditMode, LlmPatch};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt, process::Command};
use walkdir::WalkDir;

pub fn collect_context(project_root: &Path, max_bytes: usize) -> Result<String> {
    let mut buf = String::new();
    let mut total = 0usize;
    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if entry.file_type().is_dir() {
            if p.ends_with(".git") || p.ends_with("target") || p.ends_with("node_modules") {
                continue;
            }
            continue;
        }
        let rel = p.strip_prefix(project_root).unwrap_or(p);
        let rel_s = rel.to_string_lossy();
        let include = rel_s.ends_with(".rs")
            || rel_s.ends_with("Cargo.toml")
            || rel_s.starts_with("tests/")
            || rel_s.starts_with("src/")
            || rel_s.starts_with("benches/")
            || rel_s.starts_with("examples/")
            || rel_s.starts_with("README")
            || rel_s.ends_with(".md");
        if !include {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(p) else {
            continue;
        };
        let header = format!("\n===== FILE: {rel_s} =====\n");
        let needed = header.len() + contents.len();
        if total + needed > max_bytes {
            break;
        }
        buf.push_str(&header);
        buf.push_str(&contents);
        total += needed;
    }
    Ok(buf)
}

pub async fn apply_patch(project_root: &Path, patch: &LlmPatch) -> Result<Vec<PathBuf>> {
    let mut touched = Vec::new();
    for fe in &patch.files {
        let path = project_root.join(&fe.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        match fe.mode {
            EditMode::Rewrite => {
                fs::write(&path, fe.content.as_bytes()).await?;
            }
            EditMode::Append => {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await?;
                file.write_all(fe.content.as_bytes()).await?;
            }
        }
        touched.push(path);
    }
    Ok(touched)
}

pub async fn run_tests(project_root: &Path, cmd: &str) -> Result<(bool, String)> {
    // Run via shell to allow complex commands
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(cmd);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut c = Command::new("sh");
        c.arg("-lc").arg(cmd);
        c
    };

    let output = command.current_dir(project_root).output().await?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let ok = output.status.success();
    Ok((ok, text))
}
