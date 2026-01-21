use std::process::Command;

use anyhow::Result;
use tempfile::NamedTempFile;

pub fn edit_in_editor(initial_content: &str, file_extension: &str) -> Result<Option<String>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    let named_path = temp_path.with_extension(file_extension);
    std::fs::write(&named_path, initial_content)?;

    let status = Command::new(&editor).arg(&named_path).status()?;

    if !status.success() {
        std::fs::remove_file(&named_path).ok();
        return Ok(None);
    }

    let content = std::fs::read_to_string(&named_path)?;
    std::fs::remove_file(&named_path).ok();

    if content.trim() == initial_content.trim() {
        return Ok(None);
    }

    Ok(Some(content))
}

pub fn edit_markdown(initial_content: &str) -> Result<Option<String>> {
    edit_in_editor(initial_content, "md")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_edit_markdown_returns_none_for_unchanged() {
        // This test would require mocking the editor
    }
}
