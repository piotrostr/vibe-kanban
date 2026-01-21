use std::process::Command;

use anyhow::Result;

/// Send a system notification
pub fn notify(title: &str, body: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display notification "{}" with title "{}""#,
                body.replace('"', r#"\""#),
                title.replace('"', r#"\""#)
            ))
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("notify-send")
            .arg(title)
            .arg(body)
            .spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        // Windows doesn't have a simple CLI notification command
        // Could use PowerShell, but for now just skip
        tracing::debug!("Notifications not supported on Windows");
    }

    Ok(())
}
