use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SlashCommand {
    /// Command name (e.g., "commit")
    pub name: String,
    /// Qualified name including plugin namespace (e.g., "commit-commands:commit")
    pub qualified_name: String,
    /// Description from frontmatter
    pub description: Option<String>,
    /// Argument hint from frontmatter (e.g., "[file-path]")
    pub argument_hint: Option<String>,
    /// Plugin name if from a plugin
    pub plugin_name: Option<String>,
    /// Where the command comes from
    pub source: SlashCommandSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum SlashCommandSource {
    /// Built-in commands (compiled into Claude Code)
    Builtin,
    /// Commands from ~/.claude/commands/
    User,
    /// Commands from plugins
    Plugin,
}

#[derive(Debug, Deserialize)]
struct PluginJson {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
}

#[derive(Debug, Default)]
struct CommandFrontmatter {
    description: Option<String>,
    argument_hint: Option<String>,
}

/// Parse YAML frontmatter from a markdown file
fn parse_frontmatter(content: &str) -> CommandFrontmatter {
    let mut frontmatter = CommandFrontmatter::default();

    // Check if content starts with ---
    if !content.starts_with("---") {
        return frontmatter;
    }

    // Find the closing ---
    if let Some(end_idx) = content[3..].find("---") {
        let yaml_content = &content[3..3 + end_idx].trim();

        // Simple YAML parsing for our specific fields
        for line in yaml_content.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("description:") {
                frontmatter.description = Some(value.trim().trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("argument-hint:") {
                frontmatter.argument_hint = Some(value.trim().trim_matches('"').to_string());
            }
        }
    }

    frontmatter
}

/// Get command name from file path (without .md extension)
fn get_command_name(path: &std::path::Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Scan a directory for .md command files
fn scan_command_dir(
    dir: &std::path::Path,
    plugin_name: Option<&str>,
    source: SlashCommandSource,
) -> Vec<SlashCommand> {
    let mut commands = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return commands,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        let name = match get_command_name(&path) {
            Some(n) => n,
            None => continue,
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let frontmatter = parse_frontmatter(&content);

        let qualified_name = match plugin_name {
            Some(pn) => format!("{}:{}", pn, name),
            None => name.clone(),
        };

        commands.push(SlashCommand {
            name,
            qualified_name,
            description: frontmatter.description,
            argument_hint: frontmatter.argument_hint,
            plugin_name: plugin_name.map(|s| s.to_string()),
            source: source.clone(),
        });
    }

    commands
}

/// Get built-in Claude Code commands (compiled into the binary)
fn get_builtin_commands() -> Vec<SlashCommand> {
    let builtins = vec![
        ("status", "Show system and account status"),
        ("help", "Display available commands"),
        ("clear", "Clear conversation history"),
        ("config", "Open settings and configuration"),
        ("model", "Switch Claude model"),
        ("memory", "Manage conversation memory"),
        ("resume", "Resume a previous conversation"),
        ("compact", "Compact conversation to save context"),
        ("cost", "Show token usage and cost"),
        ("doctor", "Run diagnostics"),
        ("permissions", "Manage tool permissions"),
        ("vim", "Toggle vim mode"),
        ("theme", "Change syntax theme"),
        ("terminal-setup", "Configure terminal integration"),
        ("mcp", "Manage MCP servers"),
        ("export", "Export conversation"),
        ("plugin", "Manage plugins"),
        ("add-dir", "Add directory to context"),
        ("release-notes", "View release notes"),
        ("bug", "Report a bug"),
        ("logout", "Sign out of Claude"),
        ("login", "Sign in to Claude"),
        ("init", "Initialize Claude in current directory"),
        ("review", "Review code changes"),
        ("pr-comments", "View PR comments"),
    ];

    builtins
        .into_iter()
        .map(|(name, desc)| SlashCommand {
            name: name.to_string(),
            qualified_name: name.to_string(),
            description: Some(desc.to_string()),
            argument_hint: None,
            plugin_name: None,
            source: SlashCommandSource::Builtin,
        })
        .collect()
}

/// Discover all available slash commands
fn discover_commands() -> Vec<SlashCommand> {
    let mut commands = Vec::new();

    // 1. Built-in commands first
    commands.extend(get_builtin_commands());

    // 2. User commands from ~/.claude/commands/
    if let Some(home) = dirs::home_dir() {
        let user_commands_dir = home.join(".claude").join("commands");
        commands.extend(scan_command_dir(
            &user_commands_dir,
            None,
            SlashCommandSource::User,
        ));
    }

    // 3. Plugin commands - scan known plugin locations
    let plugin_dirs = get_plugin_directories();

    for plugin_dir in plugin_dirs {
        // Read plugin.json to get plugin name
        let plugin_json_path = plugin_dir.join(".claude-plugin").join("plugin.json");
        let plugin_name = match std::fs::read_to_string(&plugin_json_path) {
            Ok(content) => match serde_json::from_str::<PluginJson>(&content) {
                Ok(pj) => pj.name,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Scan commands directory
        let commands_dir = plugin_dir.join("commands");
        commands.extend(scan_command_dir(
            &commands_dir,
            Some(&plugin_name),
            SlashCommandSource::Plugin,
        ));

        // Also scan skills directory for skill commands
        let skills_dir = plugin_dir.join("skills");
        if skills_dir.exists() {
            if let Ok(skill_entries) = std::fs::read_dir(&skills_dir) {
                for skill_entry in skill_entries.flatten() {
                    let skill_path = skill_entry.path();
                    if skill_path.is_dir() {
                        // Check for skill.md in the skill directory
                        let skill_md = skill_path.join("skill.md");
                        if skill_md.exists() {
                            if let Some(skill_name) =
                                skill_path.file_name().and_then(|s| s.to_str())
                            {
                                if let Ok(content) = std::fs::read_to_string(&skill_md) {
                                    let frontmatter = parse_frontmatter(&content);
                                    let qualified_name = format!("{}:{}", plugin_name, skill_name);
                                    commands.push(SlashCommand {
                                        name: skill_name.to_string(),
                                        qualified_name,
                                        description: frontmatter.description,
                                        argument_hint: frontmatter.argument_hint,
                                        plugin_name: Some(plugin_name.clone()),
                                        source: SlashCommandSource::Plugin,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort commands alphabetically by qualified name
    commands.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    commands
}

/// Get directories containing plugins
fn get_plugin_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Check ~/.claude/plugins/ for installed plugins
    if let Some(home) = dirs::home_dir() {
        let plugins_dir = home.join(".claude").join("plugins");
        if plugins_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        dirs.push(path);
                    }
                }
            }
        }
    }

    // Also check bundled Claude Code plugins in the vibe-kanban repo
    // This handles the case where Claude Code is cloned locally
    let bundled_plugins_paths = [
        // Relative to cwd (for dev)
        PathBuf::from("claude-code/plugins"),
        // Common locations
        PathBuf::from("../claude-code/plugins"),
    ];

    for base_path in bundled_plugins_paths {
        if base_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        dirs.push(path);
                    }
                }
            }
        }
    }

    dirs
}

pub async fn list_slash_commands() -> Result<Json<ApiResponse<Vec<SlashCommand>>>, ApiError> {
    let commands = discover_commands();
    Ok(Json(ApiResponse::success(commands)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/slash-commands", get(list_slash_commands))
}
