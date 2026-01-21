use anyhow::Result;
use serde::Deserialize;
use std::process::Command;

/// PR info fetched from `gh pr view`
#[derive(Debug, Clone, Deserialize)]
pub struct BranchPrInfo {
    #[serde(rename = "number")]
    pub _number: i64,
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
    pub _typename: String,
    pub conclusion: Option<String>, // SUCCESS, FAILURE, etc.
    pub status: Option<String>,     // COMPLETED, IN_PROGRESS, etc.
}

impl BranchPrInfo {
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
        if stderr.contains("no pull requests found") || stderr.contains("no open pull requests") {
            return Ok(None);
        }
        if stderr.contains("Could not resolve") {
            return Ok(None);
        }
        anyhow::bail!("gh pr view failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let pr_info: BranchPrInfo = serde_json::from_str(&stdout)?;
    Ok(Some(pr_info))
}
