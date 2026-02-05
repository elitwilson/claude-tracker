use super::*;
use anyhow::Context;
use chrono::Utc;

// Test configuration - update these with your actual IDs
const TEST_WORKSPACE_ID: &str = "5ff748b4abb0e16bed500885";
const TEST_PROJECT_ID: &str = "65b2d73e06de527a7ed67403"; // NPR project from spike

/// Helper: Delete a time entry from Clockify (for test cleanup)
fn delete_time_entry(workspace_id: &str, entry_id: &str) -> Result<()> {
    let api_key = crate::secrets::get_secret("clockify_api_key")?;
    let url = format!(
        "https://api.clockify.me/api/v1/workspaces/{}/time-entries/{}",
        workspace_id, entry_id
    );

    ureq::delete(&url)
        .set("X-Api-Key", &api_key)
        .call()
        .context("Failed to delete test entry from Clockify")?;

    Ok(())
}

/// Happy path: POST a time entry to Clockify and verify it returns an entry ID.
/// This test actually calls the Clockify API.
/// Run with: cargo test -- --ignored
/// Cleanup: Deletes the created entry at the end.
#[test]
#[ignore]
fn test_post_time_entry_success() {
    // Create a 30-minute test entry
    let now = Utc::now();
    let start = now;
    let end = now + chrono::Duration::minutes(30);

    // POST the entry
    let result = post_time_entry(TEST_PROJECT_ID, start, end, TEST_WORKSPACE_ID);

    // Should succeed and return an entry ID
    assert!(result.is_ok(), "POST should succeed: {:?}", result.err());
    let entry_id = result.unwrap();
    assert!(!entry_id.is_empty(), "Entry ID should not be empty");

    // Clean up: delete the test entry
    let cleanup = delete_time_entry(TEST_WORKSPACE_ID, &entry_id);
    assert!(cleanup.is_ok(), "Cleanup should succeed: {:?}", cleanup.err());
}

/// Error case: POST with an invalid project ID should fail with a clear error.
#[test]
#[ignore]
fn test_post_time_entry_invalid_project() {
    let now = Utc::now();
    let start = now;
    let end = now + chrono::Duration::minutes(30);

    // POST with a bogus project ID
    let result = post_time_entry("invalid-project-id-123", start, end, TEST_WORKSPACE_ID);

    // Should fail
    assert!(result.is_err(), "POST with invalid project should fail");
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();

    // Error should mention the problem
    assert!(
        err_msg.contains("404") || err_msg.contains("not found") || err_msg.contains("project"),
        "Error should indicate project issue, got: {}",
        err
    );
}
