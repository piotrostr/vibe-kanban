#![allow(dead_code)]

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Open a new tmux pane running zellij with claude
/// This creates a vertical split in tmux and runs the zellij session there
pub fn open_tmux_pane_with_zellij_claude(session_name: &str, cwd: &Path) -> Result<()> {
    // Build the zellij command that will run claude
    let zellij_cmd = format!(
        "zellij attach {} 2>/dev/null || zellij -s {} -- claude --dangerously-skip-permissions",
        session_name,
        session_name,
    );

    // Create a new tmux pane (vertical split) and run zellij in it
    // -h = horizontal split (creates pane on right)
    // -c = start directory
    Command::new("tmux")
        .arg("split-window")
        .arg("-h")
        .arg("-c")
        .arg(cwd)
        .arg(&zellij_cmd)
        .spawn()?;

    Ok(())
}

/// Attach to existing zellij session in a new tmux pane
pub fn open_tmux_pane_attach_zellij(session_name: &str) -> Result<()> {
    let zellij_cmd = format!("zellij attach {}", session_name);

    Command::new("tmux")
        .arg("split-window")
        .arg("-h")
        .arg(&zellij_cmd)
        .spawn()?;

    Ok(())
}

// Keep the old functions for reference but rename them
/// Open a new Ghostty terminal window running zellij with claude (legacy)
#[allow(dead_code)]
pub fn open_ghostty_with_zellij_claude(session_name: &str, cwd: &Path) -> Result<()> {
    let zellij_cmd = format!(
        "zellij -s {} --cwd {} -- claude --dangerously-skip-permissions",
        session_name,
        cwd.to_string_lossy()
    );

    let script = format!(
        r#"tell application "Ghostty"
            activate
            tell application "System Events"
                keystroke "n" using command down
            end tell
            delay 0.3
            tell application "System Events"
                keystroke "{}"
                keystroke return
            end tell
        end tell"#,
        zellij_cmd.replace('"', "\\\"")
    );

    Command::new("osascript").arg("-e").arg(&script).spawn()?;

    Ok(())
}

/// Open a new Ghostty terminal window and attach to existing zellij session
pub fn open_ghostty_attach_zellij(session_name: &str) -> Result<()> {
    let zellij_cmd = format!("zellij attach {}", shell_escape(session_name));

    // Wrap in /bin/zsh -c "..." as a single string for Ghostty's -e flag
    let full_cmd = format!("/bin/zsh -c \"{}\"", zellij_cmd);

    Command::new("open")
        .arg("-na")
        .arg("Ghostty")
        .arg("--args")
        .arg("-e")
        .arg(&full_cmd)
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

/// Launch claude in a zellij session (Ctrl+b d to detach)
/// Creates session if it doesn't exist, attaches if it does
/// If task_context is provided and this is a new session, it will be passed as the initial prompt
/// If plan_mode is true, Claude will be started in plan mode
pub fn launch_zellij_claude_foreground(
    session_name: &str,
    cwd: &Path,
    task_context: Option<&str>,
    plan_mode: bool,
) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    // Try to attach first (with -f to resurrect dead sessions)
    let attach_result = Command::new("zellij")
        .args(["attach", "-f", session_name])
        .status();

    if let Ok(status) = attach_result {
        if status.success() {
            return Ok(());
        }
    }

    // Attach failed - create new session with custom shell
    // Build the claude command with optional flags
    let script_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("vibe-scripts");
    std::fs::create_dir_all(&script_dir)?;

    let script_path = script_dir.join(format!("{}.sh", session_name));

    // Build claude command with appropriate flags
    let mut claude_args = vec!["--dangerously-skip-permissions".to_string()];

    if plan_mode {
        claude_args.push("--plan".to_string());
    }

    // If we have task context, pass it as the initial prompt using --print
    let claude_cmd = if let Some(context) = task_context {
        claude_args.push("--print".to_string());
        claude_args.push(shell_escape(context));
        format!("claude {}", claude_args.join(" "))
    } else {
        claude_args.insert(0, "--continue".to_string());
        format!("claude {}", claude_args.join(" "))
    };

    let script_content = format!(
        "#!/bin/zsh\ncd {}\n{}\nexec zsh\n",
        shell_escape(&cwd.to_string_lossy()),
        claude_cmd
    );

    let mut file = std::fs::File::create(&script_path)?;
    file.write_all(script_content.as_bytes())?;
    drop(file);
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;

    let status = Command::new("zellij")
        .args(["-s", session_name])
        .env("SHELL", &script_path)
        .status()?;

    if !status.success() {
        anyhow::bail!("zellij session failed");
    }

    Ok(())
}

/// Attach to existing zellij session in current terminal (blocks)
/// Handles dead sessions by force-resurrecting them
pub fn attach_zellij_foreground(session_name: &str) -> Result<()> {
    use super::zellij::get_session_status;

    // Check if session is dead (None = doesn't exist, Some(is_dead) = exists)
    let is_dead = get_session_status(session_name).unwrap_or(false);

    let mut args = vec!["attach"];
    if is_dead {
        args.push("-f"); // Force resurrection of dead session
    }
    args.push(session_name);

    let status = Command::new("zellij").args(&args).status()?;

    if !status.success() {
        anyhow::bail!("zellij attach exited with error");
    }
    Ok(())
}
