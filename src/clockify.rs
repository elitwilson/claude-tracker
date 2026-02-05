use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for creating a Clockify time entry
#[derive(Serialize)]
struct TimeEntryRequest {
    #[serde(rename = "projectId")]
    project_id: String,
    start: String,
    end: String,
    description: String,
}

/// Response from Clockify after creating a time entry
#[derive(Deserialize)]
struct TimeEntryResponse {
    id: String,
}

/// A Clockify project from the API
#[derive(Deserialize)]
pub(crate) struct Project {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) archived: bool,
}

/// Provide helpful hints based on HTTP status code
fn status_hint(code: u16) -> &'static str {
    match code {
        400 => "invalid project ID or request parameters",
        401 => "check your API key",
        403 => "access forbidden - check workspace/project permissions",
        404 => "project or workspace not found",
        422 => "invalid request - check time range and project ID",
        _ => "unexpected error",
    }
}

/// POST a time entry to Clockify. Returns the created entry ID.
pub(crate) fn post_time_entry(
    project_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    workspace_id: &str,
) -> Result<String> {
    // Get API key from keyring
    let api_key = crate::secrets::get_secret("clockify_api_key")
        .context("Failed to retrieve Clockify API key")?;

    // Build request body
    let request = TimeEntryRequest {
        project_id: project_id.to_string(),
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        description: "Development".to_string(),
    };

    // POST to Clockify
    let url = format!(
        "https://api.clockify.me/api/v1/workspaces/{}/time-entries",
        workspace_id
    );

    let json_body = serde_json::to_string(&request)
        .context("Failed to serialize request body")?;

    let response = ureq::post(&url)
        .set("X-Api-Key", &api_key)
        .set("Content-Type", "application/json")
        .send_string(&json_body)
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => {
                anyhow::anyhow!("Clockify API returned HTTP {}: {}", code, status_hint(code))
            }
            ureq::Error::Transport(t) => {
                anyhow::anyhow!("Network error contacting Clockify: {}", t)
            }
        })?;

    // Parse response
    let response_text = response
        .into_string()
        .context("Failed to read Clockify response body")?;

    let body: TimeEntryResponse = serde_json::from_str(&response_text)
        .context("Failed to parse Clockify response JSON")?;

    Ok(body.id)
}

/// List all projects in a workspace (handles pagination)
pub(crate) fn list_projects(workspace_id: &str) -> Result<Vec<Project>> {
    let api_key = crate::secrets::get_secret("clockify_api_key")
        .context("Failed to retrieve Clockify API key")?;

    let mut all_projects = Vec::new();
    let page_size = 50;
    let mut page = 1;

    loop {
        let url = format!(
            "https://api.clockify.me/api/v1/workspaces/{}/projects?page-size={}&page={}",
            workspace_id, page_size, page
        );

        let response = ureq::get(&url)
            .set("X-Api-Key", &api_key)
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(code, _) => {
                    anyhow::anyhow!("Clockify API returned HTTP {}: {}", code, status_hint(code))
                }
                ureq::Error::Transport(t) => {
                    anyhow::anyhow!("Network error contacting Clockify: {}", t)
                }
            })?;

        let response_text = response
            .into_string()
            .context("Failed to read Clockify response body")?;

        let mut projects: Vec<Project> = serde_json::from_str(&response_text)
            .context("Failed to parse Clockify response JSON")?;

        let fetched_count = projects.len();
        all_projects.append(&mut projects);

        // If we got fewer projects than page_size, we've reached the last page
        if fetched_count < page_size {
            break;
        }

        page += 1;
    }

    Ok(all_projects)
}

#[cfg(test)]
mod tests;
