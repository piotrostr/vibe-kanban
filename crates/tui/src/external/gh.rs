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

/// PR info fetched from `gh pr view`
#[derive(Debug, Clone, Deserialize)]
pub struct BranchPrInfo {
    pub number: i64,
    pub url: String,
    pub state: String, // OPEN, CLOSED, MERGED
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    #[serde(rename = "reviewDecision")]
    pub review_decision: Option<String>, // APPROVED, CHANGES_REQUESTED, REVIEW_REQUIRED
    #[serde(rename = "statusCheckRollup")]
    pub status_check_rollup: Option<Vec<StatusCheck>>,
    #[serde(rename = "mergeable")]
    pub mergeable: Option<String>, // MERGEABLE, CONFLICTING, UNKNOWN
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusCheck {
    #[serde(rename = "__typename")]
    pub typename: String,
    pub conclusion: Option<String>, // SUCCESS, FAILURE, etc.
    pub status: Option<String>,     // COMPLETED, IN_PROGRESS, etc.
}

impl BranchPrInfo {
    /// Convert to the status string format used in Task (open, closed, merged)
    pub fn status_string(&self) -> String {
        self.state.to_lowercase()
    }

    /// Get overall checks status: SUCCESS, FAILURE, PENDING, or None
    pub fn checks_status(&self) -> Option<String> {
        let checks = self.status_check_rollup.as_ref()?;
        if checks.is_empty() {
            return None;
        }

        let mut has_failure = false;
        let mut has_pending = false;

        for check in checks {
            match check.conclusion.as_deref() {
                Some("FAILURE") | Some("ERROR") | Some("TIMED_OUT") => has_failure = true,
                Some("SUCCESS") | Some("NEUTRAL") | Some("SKIPPED") => {}
                _ => {
                    // No conclusion yet or still running
                    if check.status.as_deref() != Some("COMPLETED") {
                        has_pending = true;
                    }
                }
            }
        }

        if has_failure {
            Some("FAILURE".to_string())
        } else if has_pending {
            Some("PENDING".to_string())
        } else {
            Some("SUCCESS".to_string())
        }
    }

    /// Check if PR has merge conflicts
    pub fn has_conflicts(&self) -> bool {
        self.mergeable.as_deref() == Some("CONFLICTING")
    }
}

/// Get PR info for a specific branch using `gh pr view`
/// Returns None if no PR exists for the branch
pub fn get_pr_for_branch(branch: &str) -> Result<Option<BranchPrInfo>> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            branch,
            "--json",
            "number,url,state,isDraft,reviewDecision,statusCheckRollup,mergeable",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // "no pull requests found" is not an error, just means no PR exists
        if stderr.contains("no pull requests found") || stderr.contains("no open pull requests") {
            return Ok(None);
        }
        // GraphQL errors for missing fields are also okay
        if stderr.contains("Could not resolve") {
            return Ok(None);
        }
        anyhow::bail!("gh pr view failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let pr_info: BranchPrInfo = serde_json::from_str(&stdout)?;
    Ok(Some(pr_info))
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
