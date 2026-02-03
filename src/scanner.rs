use std::fs;
use std::path::{Path, PathBuf};

/// Find all session JSONL files under a Claude projects directory.
/// Skips agent-* files and bare subdirectories.
pub fn find_session_files(projects_dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let project_dirs = match fs::read_dir(projects_dir) {
        Ok(entries) => entries,
        Err(_) => return results,
    };

    for project_entry in project_dirs.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let files = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for file_entry in files.flatten() {
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }

            if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".jsonl") && !name.starts_with("agent-") {
                    results.push(file_path);
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests;
