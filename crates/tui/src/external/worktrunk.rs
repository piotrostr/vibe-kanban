#![allow(dead_code)]

use anyhow::Result;
use serde::Deserialize;
use std::process::Command;

/// Get the wt binary path - check WORKTRUNK_BIN env or fall back to cargo bin
fn wt_binary() -> String {
    std::env::var("WORKTRUNK_BIN").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".cargo/bin/wt").to_string_lossy().to_string())
            .unwrap_or_else(|| "wt".to_string())
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeInfo {
    pub branch: String,
    pub path: String,
    #[serde(default)]
    pub kind: String,
    pub commit: Option<CommitInfo>,
    pub working_tree: Option<WorkingTreeStatus>,
    #[serde(default)]
    pub main_state: String,
    pub main: Option<MainStatus>,
    #[serde(default)]
    pub is_main: bool,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default)]
    pub is_previous: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkingTreeStatus {
    #[serde(default)]
    pub staged: bool,
    #[serde(default)]
    pub modified: bool,
    #[serde(default)]
    pub untracked: bool,
    pub diff: Option<DiffStats>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiffStats {
    #[serde(default)]
    pub added: i32,
    #[serde(default)]
    pub deleted: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainStatus {
    #[serde(default)]
    pub ahead: i32,
    #[serde(default)]
    pub behind: i32,
}

impl WorktreeInfo {
    pub fn status_symbol(&self) -> &'static str {
        match self.main_state.as_str() {
            "ahead" => "+",
            "behind" => "-",
            "diverged" => "!",
            "empty" => "=",
            _ => " ",
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.working_tree
            .as_ref()
            .map(|wt| wt.staged || wt.modified || wt.untracked)
            .unwrap_or(false)
    }

    pub fn short_commit(&self) -> &str {
        self.commit
            .as_ref()
            .map(|c| c.short_sha.as_str())
            .unwrap_or("-------")
    }
}

pub fn list_worktrees() -> Result<Vec<WorktreeInfo>> {
    let output = Command::new(wt_binary())
        .args(["list", "--format=json"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wt list failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let worktrees: Vec<WorktreeInfo> = serde_json::from_str(&stdout)?;
    Ok(worktrees)
}

pub fn create_worktree(branch: &str) -> Result<()> {
    let status = Command::new(wt_binary())
        .args(["switch", "--create", branch])
        .status()?;

    if !status.success() {
        anyhow::bail!("wt switch --create {} failed", branch);
    }
    Ok(())
}

pub fn switch_worktree(branch: &str) -> Result<()> {
    let status = Command::new(wt_binary())
        .args(["switch", branch])
        .status()?;

    if !status.success() {
        anyhow::bail!("wt switch {} failed", branch);
    }
    Ok(())
}

pub fn remove_worktree() -> Result<()> {
    let status = Command::new(wt_binary()).args(["remove"]).status()?;

    if !status.success() {
        anyhow::bail!("wt remove failed");
    }
    Ok(())
}

pub fn get_current_worktree() -> Result<Option<WorktreeInfo>> {
    let worktrees = list_worktrees()?;
    Ok(worktrees.into_iter().find(|wt| wt.is_current))
}
