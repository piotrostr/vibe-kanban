use anyhow::Result;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ZellijSession {
    pub name: String,
    pub is_current: bool,
}

pub fn list_sessions() -> Result<Vec<ZellijSession>> {
    let output = Command::new("zellij").args(["list-sessions"]).output()?;

    if !output.status.success() {
        // zellij returns error if no sessions exist
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No active sessions") || stderr.is_empty() {
            return Ok(Vec::new());
        }
        anyhow::bail!("zellij list-sessions failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let sessions: Vec<ZellijSession> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Format: "session-name (current)" or just "session-name"
            let is_current = line.contains("(current)");
            let name = line
                .trim()
                .trim_end_matches("(current)")
                .trim()
                .to_string();
            ZellijSession { name, is_current }
        })
        .collect();

    Ok(sessions)
}

pub fn session_exists(name: &str) -> bool {
    list_sessions()
        .map(|sessions| sessions.iter().any(|s| s.name == name))
        .unwrap_or(false)
}

pub fn create_session_with_command(name: &str, cwd: &Path, command: &str) -> Result<()> {
    // Create a zellij session that runs the specified command
    // We use `zellij -s <name> --cwd <path> -- <command>`
    let status = Command::new("zellij")
        .arg("-s")
        .arg(name)
        .arg("--cwd")
        .arg(cwd)
        .arg("--")
        .arg(command)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to create zellij session: {}", name);
    }
    Ok(())
}

pub fn attach_session(name: &str) -> Result<()> {
    let status = Command::new("zellij")
        .args(["attach", name])
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to attach to zellij session: {}", name);
    }
    Ok(())
}

pub fn kill_session(name: &str) -> Result<()> {
    let status = Command::new("zellij")
        .args(["kill-session", name])
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to kill zellij session: {}", name);
    }
    Ok(())
}

pub fn sanitize_session_name(branch: &str) -> String {
    // Convert branch name to valid zellij session name
    // Replace slashes and special chars with dashes
    branch
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn session_name_for_branch(branch: &str) -> String {
    format!("vibe-{}", sanitize_session_name(branch))
}

pub fn is_zellij_installed() -> bool {
    Command::new("zellij")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
