use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Open a new Ghostty terminal window running zellij with claude
pub fn open_ghostty_with_zellij_claude(session_name: &str, cwd: &Path) -> Result<()> {
    // Build the zellij command that will run claude
    // Format: zellij -s <session> --cwd <path> -- claude --dangerously-skip-permissions
    let zellij_cmd = format!(
        "zellij -s {} --cwd {} -- claude --dangerously-skip-permissions",
        shell_escape(session_name),
        shell_escape(&cwd.to_string_lossy())
    );

    // Open Ghostty with the command wrapped in /bin/zsh -c
    // This is needed because Ghostty's -e flag passes through /usr/bin/login
    // which doesn't handle complex arguments with -- separators correctly
    Command::new("open")
        .arg("-na")
        .arg("Ghostty")
        .arg("--args")
        .arg("-e")
        .arg("/bin/zsh")
        .arg("-c")
        .arg(&zellij_cmd)
        .spawn()?;

    Ok(())
}

/// Open a new Ghostty terminal window and attach to existing zellij session
pub fn open_ghostty_attach_zellij(session_name: &str) -> Result<()> {
    let zellij_cmd = format!("zellij attach {}", shell_escape(session_name));

    // Wrap in /bin/zsh -c for consistent behavior with Ghostty
    Command::new("open")
        .arg("-na")
        .arg("Ghostty")
        .arg("--args")
        .arg("-e")
        .arg("/bin/zsh")
        .arg("-c")
        .arg(&zellij_cmd)
        .spawn()?;

    Ok(())
}

/// Open a new Ghostty terminal window with a custom command
pub fn open_ghostty_with_command(command: &str, cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("open");
    cmd.arg("-na").arg("Ghostty").arg("--args").arg("-e");

    if let Some(dir) = cwd {
        // Wrap command to cd first
        let full_cmd = format!("cd {} && {}", shell_escape(&dir.to_string_lossy()), command);
        cmd.arg(&full_cmd);
    } else {
        cmd.arg(command);
    }

    cmd.spawn()?;
    Ok(())
}

/// Simple shell escape for command arguments
fn shell_escape(s: &str) -> String {
    // If string contains no special chars, return as-is
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
    {
        return s.to_string();
    }
    // Otherwise, wrap in single quotes and escape any single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Launch zellij session with claude in current terminal (blocks)
/// This suspends the TUI and gives control to zellij
pub fn launch_zellij_claude_foreground(session_name: &str, cwd: &Path) -> Result<()> {
    let status = Command::new("zellij")
        .arg("-s")
        .arg(session_name)
        .arg("--cwd")
        .arg(cwd)
        .arg("--")
        .arg("claude")
        .arg("--dangerously-skip-permissions")
        .status()?;

    if !status.success() {
        anyhow::bail!("zellij session exited with error");
    }
    Ok(())
}

/// Attach to existing zellij session in current terminal (blocks)
pub fn attach_zellij_foreground(session_name: &str) -> Result<()> {
    let status = Command::new("zellij")
        .args(["attach", session_name])
        .status()?;

    if !status.success() {
        anyhow::bail!("zellij attach exited with error");
    }
    Ok(())
}
