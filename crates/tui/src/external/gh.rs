use anyhow::Result;
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Deserialize)]
pub struct PrListItem {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub author: PrAuthor,
    #[serde(rename = "headRefName")]
    #[allow(dead_code)]
    pub head_ref_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrAuthor {
    pub login: String,
}

/// List recent PRs using gh CLI
pub fn list_prs(limit: u32, search: Option<&str>) -> Result<Vec<PrListItem>> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "pr",
        "list",
        "--json",
        "number,title,state,author,headRefName",
        "--limit",
        &limit.to_string(),
        "--state",
        "all",
    ]);

    if let Some(query) = search {
        cmd.args(["--search", query]);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr list failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let prs: Vec<PrListItem> = serde_json::from_str(&stdout)?;
    Ok(prs)
}

/// Select a PR using fzf
/// Returns the selected PR number, or None if cancelled
pub fn select_pr_with_fzf(prs: &[PrListItem]) -> Result<Option<i64>> {
    if prs.is_empty() {
        return Ok(None);
    }

    // Format PRs for fzf display: "#123 [OPEN] title (author)"
    let lines: Vec<String> = prs
        .iter()
        .map(|pr| {
            format!(
                "#{} [{}] {} (@{})",
                pr.number,
                pr.state.to_uppercase(),
                pr.title,
                pr.author.login
            )
        })
        .collect();

    let input = lines.join("\n");

    // Run fzf with the PR list
    let mut child = Command::new("fzf")
        .args([
            "--height=40%",
            "--reverse",
            "--prompt=Select PR> ",
            "--header=Press ESC to cancel",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Write PR list to fzf stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        // fzf returns non-zero when user cancels with ESC
        return Ok(None);
    }

    let selection = String::from_utf8(output.stdout)?;
    let selection = selection.trim();

    if selection.is_empty() {
        return Ok(None);
    }

    // Parse PR number from "#123 [OPEN] title..."
    if let Some(num_str) = selection.strip_prefix('#') {
        if let Some(space_idx) = num_str.find(' ') {
            if let Ok(num) = num_str[..space_idx].parse::<i64>() {
                return Ok(Some(num));
            }
        }
    }

    Ok(None)
}
