use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ZellijSession {
    pub name: String,
    pub is_current: bool,
    pub is_dead: bool,
    pub needs_attention: bool,
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
            // Format: "session-name [Created 3m 5s ago] (current)"
            // Or dead: "session-name [Created 3m 5s ago] (EXITED -9attach to resurrect)"
            let is_current = line.contains("(current)");
            let is_dead = line.contains("EXITED");

            // Extract session name: everything before first '[' or space with metadata
            let name = line
                .split(|c| c == '[')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            ZellijSession {
                name,
                is_current,
                is_dead,
                needs_attention: false,
            }
        })
        .collect();

    Ok(sessions)
}

/// Check if a session is waiting for user input by dumping screen content
pub fn check_session_needs_attention(session_name: &str) -> bool {
    // Dump the last few lines of the session screen
    let output = Command::new("zellij")
        .args([
            "action",
            "--session",
            session_name,
            "dump-screen",
            "/dev/stdout",
        ])
        .output();

    let Ok(output) = output else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    let screen = String::from_utf8_lossy(&output.stdout);
    let last_lines: String = screen.lines().rev().take(10).collect::<Vec<_>>().join("\n");

    // Patterns that indicate Claude is waiting for input
    let attention_patterns = [
        "? ",                            // Interactive prompt
        "[y/n]",                         // Yes/no prompt
        "(y/N)",                         // Yes/no with default
        "(Y/n)",                         // Yes/no with default
        "Continue?",                     // Confirmation
        "Press Enter",                   // Waiting for enter
        "Proceed?",                      // Confirmation
        "Do you want to",                // Confirmation question
        ">",                             // Generic prompt at end of line
        "waiting for",                   // Waiting state
        "permission",                    // Permission request
    ];

    attention_patterns
        .iter()
        .any(|pattern| last_lines.to_lowercase().contains(&pattern.to_lowercase()))
}

/// List sessions with attention status (slower, checks each session)
pub fn list_sessions_with_status() -> Result<Vec<ZellijSession>> {
    let mut sessions = list_sessions()?;
    for session in &mut sessions {
        session.needs_attention = check_session_needs_attention(&session.name);
    }
    Ok(sessions)
}

pub fn session_exists(name: &str) -> bool {
    list_sessions()
        .map(|sessions| sessions.iter().any(|s| s.name == name))
        .unwrap_or(false)
}

/// Check if a session exists and whether it's dead (needs resurrection)
/// Returns None if session doesn't exist, Some(is_dead) if it does
pub fn get_session_status(name: &str) -> Option<bool> {
    list_sessions()
        .ok()
        .and_then(|sessions| sessions.iter().find(|s| s.name == name).map(|s| s.is_dead))
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
    attach_session_with_resurrect(name, false)
}

/// Attach to a session, optionally forcing resurrection of dead sessions
pub fn attach_session_with_resurrect(name: &str, force_resurrect: bool) -> Result<()> {
    let mut args = vec!["attach"];
    if force_resurrect {
        args.push("-f"); // Force resurrection of dead session
    }
    args.push(name);

    let status = Command::new("zellij").args(&args).status()?;

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
    sanitize_session_name(branch)
}

pub fn is_zellij_installed() -> bool {
    Command::new("zellij")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the zellij config directory
fn zellij_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("zellij")
}

/// Vibe-specific zellij config that:
/// - Unbinds Ctrl+p and Ctrl+n to let bash handle history navigation
/// - Binds Ctrl+q for quick detach
/// - Hides the status bar for a cleaner look
const VIBE_ZELLIJ_CONFIG: &str = r#"// Vibe TUI zellij config
// Unbinds common bash shortcuts to allow history navigation

// Skip intro screen
on_force_close "detach"

// Hide the mode/status bar at the bottom for a cleaner look
simplified_ui true
pane_frames false
default_layout "compact"

keybinds clear-defaults=true {
    // Unbind Ctrl+p and Ctrl+n to let bash handle them for history
    // These are not bound to anything, so they pass through

    normal {
        // Quick detach with Ctrl+q
        bind "Ctrl q" { Detach; }

        // Basic navigation
        bind "Alt h" "Alt Left" { MoveFocusOrTab "Left"; }
        bind "Alt l" "Alt Right" { MoveFocusOrTab "Right"; }
        bind "Alt j" "Alt Down" { MoveFocus "Down"; }
        bind "Alt k" "Alt Up" { MoveFocus "Up"; }

        // Scrolling
        bind "PageUp" { ScrollUp; }
        bind "PageDown" { ScrollDown; }

        // Enter scroll mode
        bind "Ctrl s" { SwitchToMode "Scroll"; }

        // Copy mode
        bind "Ctrl c" { SwitchToMode "EnterSearch"; SearchInput 0; }
    }

    scroll {
        bind "j" "Down" { ScrollDown; }
        bind "k" "Up" { ScrollUp; }
        bind "d" { HalfPageScrollDown; }
        bind "u" { HalfPageScrollUp; }
        bind "Ctrl d" { HalfPageScrollDown; }
        bind "Ctrl u" { HalfPageScrollUp; }
        bind "Esc" "q" { SwitchToMode "Normal"; }
    }

    entersearch {
        bind "Esc" { SwitchToMode "Scroll"; }
        bind "Enter" { SwitchToMode "Search"; }
    }

    search {
        bind "n" { Search "down"; }
        bind "N" { Search "up"; }
        bind "Esc" "q" { SwitchToMode "Normal"; }
    }
}
"#;

/// Ensure zellij config exists with vibe-specific settings
/// Returns true if config was created, false if it already exists
pub fn ensure_zellij_config() -> Result<bool> {
    let config_dir = zellij_config_dir();
    let config_path = config_dir.join("config.kdl");

    if config_path.exists() {
        // Config already exists - don't overwrite user's config
        return Ok(false);
    }

    // Create config directory if it doesn't exist
    fs::create_dir_all(&config_dir)?;

    // Write the vibe config
    fs::write(&config_path, VIBE_ZELLIJ_CONFIG)?;

    Ok(true)
}
