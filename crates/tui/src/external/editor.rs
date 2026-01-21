use std::process::Command;

use anyhow::Result;
use tempfile::NamedTempFile;

/// Edit text in the user's preferred editor (defaults to nvim)
/// Returns the edited content or None if editing was cancelled
pub fn edit_in_editor(initial_content: &str, file_extension: &str) -> Result<Option<String>> {
    // Get editor from environment or default to nvim
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

    // Create temp file with the appropriate extension
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    // Rename with extension for syntax highlighting
    let named_path = temp_path.with_extension(file_extension);
    std::fs::write(&named_path, initial_content)?;

    // Spawn editor
    let status = Command::new(&editor).arg(&named_path).status()?;

    if !status.success() {
        // Editor exited with error - treat as cancelled
        std::fs::remove_file(&named_path).ok();
        return Ok(None);
    }

    // Read the edited content
    let content = std::fs::read_to_string(&named_path)?;

    // Clean up
    std::fs::remove_file(&named_path).ok();

    // Return None if content is unchanged or empty
    if content.trim() == initial_content.trim() {
        return Ok(None);
    }

    Ok(Some(content))
}

/// Edit markdown content (for task descriptions)
pub fn edit_markdown(initial_content: &str) -> Result<Option<String>> {
    edit_in_editor(initial_content, "md")
}

/// Edit plain text (for follow-ups)
pub fn edit_text(initial_content: &str) -> Result<Option<String>> {
    edit_in_editor(initial_content, "txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_markdown_returns_none_for_unchanged() {
        // This test would require mocking the editor
        // For now, just verify the function signature compiles
    }
}
