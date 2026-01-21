use anyhow::{Context, Result};
use rusqlite::Connection;
use uuid::Uuid;

fn main() -> Result<()> {
    let vibe_dir = dirs::home_dir()
        .context("No home directory")?
        .join(".vibe");

    let db_path = vibe_dir.join("db.sqlite");

    if !db_path.exists() {
        println!("No database found at {:?}", db_path);
        println!("Nothing to migrate.");
        return Ok(());
    }

    println!("Migrating tasks from {:?}", db_path);

    let conn = Connection::open(&db_path)?;

    // Get all projects
    let mut projects_stmt = conn.prepare("SELECT id, name FROM projects")?;
    let projects: Vec<(Vec<u8>, String)> = projects_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    if projects.is_empty() {
        println!("No projects found in database.");
        return Ok(());
    }

    println!("Found {} projects", projects.len());

    let mut total_migrated = 0;

    for (project_id_blob, project_name) in &projects {
        let tasks_dir = vibe_dir
            .join("projects")
            .join(project_name)
            .join("tasks");

        std::fs::create_dir_all(&tasks_dir)?;

        // Get tasks for this project
        let mut tasks_stmt = conn.prepare(
            "SELECT id, title, description, linear_issue_id, linear_url, linear_labels, created_at
             FROM tasks
             WHERE project_id = ?",
        )?;

        let tasks: Vec<TaskRow> = tasks_stmt
            .query_map([project_id_blob], |row| {
                let id_blob: Vec<u8> = row.get(0)?;
                let id = Uuid::from_slice(&id_blob)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                Ok(TaskRow {
                    id,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    linear_issue_id: row.get(3)?,
                    linear_url: row.get(4)?,
                    linear_labels: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        if tasks.is_empty() {
            continue;
        }

        println!(
            "  Project '{}': {} tasks",
            project_name,
            tasks.len()
        );

        for task in tasks {
            let slug = slugify(&task.title);
            let file_path = tasks_dir.join(format!("{}.md", slug));

            // Skip if file already exists
            if file_path.exists() {
                println!("    Skipping '{}' (already exists)", task.title);
                continue;
            }

            let content = build_task_content(&task);
            std::fs::write(&file_path, content)?;
            println!("    Migrated: {}", task.title);
            total_migrated += 1;
        }
    }

    println!("\nMigration complete. {} tasks migrated.", total_migrated);
    Ok(())
}

struct TaskRow {
    id: String,
    title: String,
    description: Option<String>,
    linear_issue_id: Option<String>,
    linear_url: Option<String>,
    linear_labels: Option<String>,
    created_at: String,
}

fn build_task_content(task: &TaskRow) -> String {
    let mut frontmatter = vec![format!("id: {}", task.id)];

    if let Some(ref linear_id) = task.linear_issue_id {
        frontmatter.push(format!("linear_id: {}", linear_id));
    }

    if let Some(ref linear_url) = task.linear_url {
        frontmatter.push(format!("linear_url: {}", linear_url));
    }

    if let Some(ref labels) = task.linear_labels {
        if !labels.is_empty() && labels != "[]" {
            frontmatter.push(format!("linear_labels: {}", labels));
        }
    }

    // Parse the created_at timestamp and format as date
    let created_date = task
        .created_at
        .split('T')
        .next()
        .unwrap_or(&task.created_at);
    frontmatter.push(format!("created: {}", created_date));

    let description = task.description.as_deref().unwrap_or("");

    format!(
        "---\n{}\n---\n\n# {}\n\n{}",
        frontmatter.join("\n"),
        task.title,
        description
    )
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
