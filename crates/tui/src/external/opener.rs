use std::process::Command;

use anyhow::Result;

/// Open a URL in the system default browser
pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
    }

    Ok(())
}

/// Open a file in the system default application
pub fn open_file(path: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", path]).spawn()?;
    }

    Ok(())
}
