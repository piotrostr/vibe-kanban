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

/// Create a launcher script for zellij session
/// fresh_cmd: command for new sessions (with prompt)
/// continue_cmd: command for resuming EXITED sessions (with --continue)
fn create_launcher_script(
    session_name: &str,
    fresh_cmd: &str,
    continue_cmd: &str,
) -> Result<std::path::PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let script_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("vibe-scripts");
    std::fs::create_dir_all(&script_dir)?;

    // Shell script for fresh sessions (with prompt)
    let fresh_script_path = script_dir.join(format!("{}-fresh.sh", session_name));
    let fresh_script = format!("#!/bin/zsh\nexec {}\n", fresh_cmd);
    let mut file = std::fs::File::create(&fresh_script_path)?;
    file.write_all(fresh_script.as_bytes())?;
    drop(file);
    std::fs::set_permissions(&fresh_script_path, std::fs::Permissions::from_mode(0o755))?;

    // Shell script for continuing sessions (with --continue)
    let continue_script_path = script_dir.join(format!("{}-continue.sh", session_name));
    let continue_script = format!("#!/bin/zsh\nexec {}\n", continue_cmd);
    let mut file = std::fs::File::create(&continue_script_path)?;
    file.write_all(continue_script.as_bytes())?;
    drop(file);
    std::fs::set_permissions(&continue_script_path, std::fs::Permissions::from_mode(0o755))?;

    // Launcher script that wt switch -x will execute
    // Check session state and handle accordingly:
    // - Running: attach to it
    // - EXITED: delete and create with --continue (resume conversation)
    // - Not found: create new with prompt
    let launcher_path = script_dir.join(format!("{}-launch.sh", session_name));
    let launcher_script = format!(
        r#"#!/bin/zsh
# Strip ANSI color codes for reliable grep
SESSION_LINE=$(zellij list-sessions 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g' | grep "^{0}")
if [[ -n "$SESSION_LINE" ]]; then
  if echo "$SESSION_LINE" | grep -q "EXITED"; then
    zellij delete-session {0} 2>/dev/null
    SHELL={2} exec zellij -s {0}
  else
    exec zellij attach {0}
  fi
fi
SHELL={1} exec zellij -s {0}
"#,
        session_name,
        fresh_script_path.display(),
        continue_script_path.display()
    );
    let mut file = std::fs::File::create(&launcher_path)?;
    file.write_all(launcher_script.as_bytes())?;
    drop(file);
    std::fs::set_permissions(&launcher_path, std::fs::Permissions::from_mode(0o755))?;

    Ok(launcher_path)
}

/// Launch claude in a zellij session for an existing worktree
/// Uses `wt switch` to ensure we're in the right worktree, then launches zellij
pub fn launch_zellij_claude_in_worktree(branch: &str, plan_mode: bool) -> Result<()> {
    let session_name = super::session_name_for_branch(branch);

    // Both fresh and continue use --continue since this function is for existing worktrees
    let claude_cmd = if plan_mode {
        "claude --continue --dangerously-skip-permissions --plan"
    } else {
        "claude --continue --dangerously-skip-permissions"
    };

    let launcher = create_launcher_script(&session_name, claude_cmd, claude_cmd)?;
    let launcher_path = launcher.to_str().unwrap();

    // Try wt switch first (branch exists), fall back to --create (new branch)
    let status = Command::new("wt")
        .args(["switch", branch, "-y", "-x", launcher_path])
        .status();

    if status.is_err() || !status.unwrap().success() {
        // Branch doesn't exist, create it
        let status = Command::new("wt")
            .args(["switch", "--create", branch, "-y", "-x", launcher_path])
            .status()?;

        if !status.success() {
            anyhow::bail!("wt switch failed");
        }
    }

    Ok(())
}

/// Launch claude in a zellij session with task context for fresh tasks
/// Creates worktree if needed, passes task context as initial prompt
pub fn launch_zellij_claude_in_worktree_with_context(
    branch: &str,
    task_context: &str,
    plan_mode: bool,
) -> Result<()> {
    let session_name = super::session_name_for_branch(branch);

    // Create launcher script with task context (handles attach to existing or create new)
    // Write task context to a temp file to avoid escaping issues
    let context_file = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("vibe-scripts")
        .join(format!("{}-context.txt", session_name));
    std::fs::create_dir_all(context_file.parent().unwrap())?;
    std::fs::write(&context_file, task_context)?;

    // Fresh: pass prompt as positional argument for interactive session
    // Continue: use --continue for EXITED sessions (conversation already exists)
    let (fresh_cmd, continue_cmd) = if plan_mode {
        (
            format!(
                "claude --dangerously-skip-permissions --plan \"$(cat {})\"",
                context_file.display()
            ),
            "claude --continue --dangerously-skip-permissions --plan".to_string(),
        )
    } else {
        (
            format!(
                "claude --dangerously-skip-permissions \"$(cat {})\"",
                context_file.display()
            ),
            "claude --continue --dangerously-skip-permissions".to_string(),
        )
    };

    let launcher = create_launcher_script(&session_name, &fresh_cmd, &continue_cmd)?;
    let launcher_path = launcher.to_str().unwrap();

    // Try wt switch first (branch exists), fall back to --create (new branch)
    let status = Command::new("wt")
        .args(["switch", branch, "-y", "-x", launcher_path])
        .status();

    if status.is_err() || !status.unwrap().success() {
        // Branch doesn't exist, create it
        let status = Command::new("wt")
            .args(["switch", "--create", branch, "-y", "-x", launcher_path])
            .status()?;

        if !status.success() {
            anyhow::bail!("wt switch failed");
        }
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
