use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc, Weekday};
use std::collections::HashMap;

use crate::parser;
use crate::store::Store;
use crate::SyncConfig;

pub(crate) struct Allocation {
    pub(crate) project_id: String,
    pub(crate) start: DateTime<Utc>,
    pub(crate) end: DateTime<Utc>,
}

pub(crate) struct AllocResult {
    pub(crate) allocations: Vec<Allocation>,
    pub(crate) skipped: Vec<String>,
}

/// Sessions for one day + config + date → allocations.
pub(crate) fn allocate(
    sessions: &[parser::Session],
    config: &SyncConfig,
    date: NaiveDate,
) -> Result<AllocResult> {
    if sessions.is_empty() {
        return Ok(AllocResult {
            allocations: vec![],
            skipped: vec![],
        });
    }
    let (start, end) = work_day_boundaries(&config.work_day_start, &config.work_day_end, date)?;
    Ok(compute_allocations(
        sessions,
        &config.project_mapping,
        &config.other_project_id,
        start,
        end,
    ))
}

/// Parse work_day_start/end strings and convert to UTC for a given date.
fn work_day_boundaries(
    start: &str,
    end: &str,
    date: NaiveDate,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start_time =
        NaiveTime::parse_from_str(start, "%H:%M").context("parsing work_day_start")?;
    let end_time = NaiveTime::parse_from_str(end, "%H:%M").context("parsing work_day_end")?;
    let start_utc = Local
        .from_local_datetime(&date.and_time(start_time))
        .single()
        .context("ambiguous local time for work_day_start")?
        .with_timezone(&Utc);
    let end_utc = Local
        .from_local_datetime(&date.and_time(end_time))
        .single()
        .context("ambiguous local time for work_day_end")?
        .with_timezone(&Utc);

    Ok((start_utc, end_utc))
}

/// Core allocation logic. Pure: operates on pre-converted UTC boundaries.
fn compute_allocations(
    sessions: &[parser::Session],
    project_mapping: &HashMap<String, String>,
    other_project_id: &Option<String>,
    work_day_start: DateTime<Utc>,
    work_day_end: DateTime<Utc>,
) -> AllocResult {
    if sessions.is_empty() {
        return AllocResult {
            allocations: vec![],
            skipped: vec![],
        };
    }

    // Step 1: Build buckets - sum duration per project ID, track skipped
    let mut buckets: HashMap<String, i64> = HashMap::new();
    let mut skipped = Vec::new();
    let mut total_included = 0i64; // Only count time that will be allocated

    for session in sessions {
        let duration_secs = session.duration.num_seconds();

        if let Some(project_id) = project_mapping.get(&session.project) {
            // Mapped project
            *buckets.entry(project_id.clone()).or_insert(0) += duration_secs;
            total_included += duration_secs;
        } else {
            // Unmapped project
            if let Some(other_id) = other_project_id {
                *buckets.entry(other_id.clone()).or_insert(0) += duration_secs;
                total_included += duration_secs;
            } else {
                // Other disabled - skip this project
                if !skipped.contains(&session.project) {
                    skipped.push(session.project.clone());
                }
            }
        }
    }

    if buckets.is_empty() {
        // All sessions were unmapped and other is disabled
        return AllocResult {
            allocations: vec![],
            skipped,
        };
    }

    // Step 2: Calculate ratios and allocate work day proportionally
    let work_day_secs = (work_day_end - work_day_start).num_seconds();
    let mut allocations_with_durations: Vec<(String, i64)> = buckets
        .iter()
        .map(|(project_id, tracked_secs)| {
            let ratio = *tracked_secs as f64 / total_included as f64;
            let allocated_secs = (work_day_secs as f64 * ratio).floor() as i64;
            (project_id.clone(), allocated_secs)
        })
        .collect();

    // Sort by project_id for consistent ordering
    allocations_with_durations.sort_by(|a, b| a.0.cmp(&b.0));

    // Step 3: Adjust last entry to absorb rounding remainder
    let sum_allocated: i64 = allocations_with_durations.iter().map(|(_, d)| d).sum();
    let remainder = work_day_secs - sum_allocated;
    if let Some(last) = allocations_with_durations.last_mut() {
        last.1 += remainder;
    }

    // Step 4: Create contiguous allocations from work_day_start
    let mut current_start = work_day_start;
    let allocations = allocations_with_durations
        .into_iter()
        .map(|(project_id, duration)| {
            let end = current_start + TimeDelta::seconds(duration);
            let alloc = Allocation {
                project_id,
                start: current_start,
                end,
            };
            current_start = end;
            alloc
        })
        .collect();

    AllocResult {
        allocations,
        skipped,
    }
}

/// Check if a date is a weekday (Mon-Fri)
pub(crate) fn is_weekday(date: NaiveDate) -> bool {
    matches!(
        date.weekday(),
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
    )
}

/// Run the sync loop: process all unsynced workdays from earliest session to yesterday
pub fn run_sync(store: &Store, config: &SyncConfig) -> Result<()> {
    // Get earliest session date
    let start_date = match store.earliest_session_date()? {
        Some(date) => date,
        None => {
            println!("No sessions found. Nothing to sync.");
            return Ok(());
        }
    };

    // End date is yesterday (today's work day isn't complete yet)
    let today = Local::now().date_naive();
    let yesterday = today.pred_opt().context("Failed to compute yesterday")?;

    if start_date > yesterday {
        println!("No complete workdays to sync.");
        return Ok(());
    }

    println!("Syncing workdays from {} to {}...", start_date, yesterday);

    let mut total_days = 0;
    let mut total_entries = 0;

    // Iterate over all dates from start to yesterday
    let mut current_date = start_date;
    while current_date <= yesterday {
        // Skip weekends
        if !is_weekday(current_date) {
            current_date = current_date.succ_opt().context("Date overflow")?;
            continue;
        }

        // Check if day is already synced
        let date_str = current_date.format("%Y-%m-%d").to_string();
        if store.is_day_synced(&date_str, &config.workspace_id)? {
            current_date = current_date.succ_opt().context("Date overflow")?;
            continue;
        }

        // Get UTC boundaries for this day
        let (start_utc, end_utc) = work_day_boundaries(
            &config.work_day_start,
            &config.work_day_end,
            current_date,
        )?;

        // Query sessions for this day
        let sessions = store.query_range(start_utc, end_utc)?;

        // Skip days with zero sessions (don't mark as synced)
        if sessions.is_empty() {
            current_date = current_date.succ_opt().context("Date overflow")?;
            continue;
        }

        // Transform sessions → allocations
        let alloc_result = allocate(&sessions, config, current_date)?;

        if alloc_result.allocations.is_empty() {
            println!("  {} - no allocations (all projects skipped)", date_str);
            current_date = current_date.succ_opt().context("Date overflow")?;
            continue;
        }

        // POST each allocation
        print!("  {} - syncing", date_str);
        let mut day_entries = 0;

        for allocation in &alloc_result.allocations {
            // Check per-entry idempotency
            if store.is_entry_synced(&date_str, &config.workspace_id, &allocation.project_id)? {
                continue;
            }

            // POST to Clockify
            let entry_id = crate::clockify::post_time_entry(
                &allocation.project_id,
                allocation.start,
                allocation.end,
                &config.workspace_id,
            )?;

            // Record entry
            store.mark_entry_synced(&date_str, &config.workspace_id, &allocation.project_id, &entry_id)?;
            day_entries += 1;
        }

        // Mark day complete
        store.mark_day_synced(&date_str, &config.workspace_id)?;

        println!(" - {} entries posted", day_entries);
        total_days += 1;
        total_entries += day_entries;

        current_date = current_date.succ_opt().context("Date overflow")?;
    }

    println!("---");
    println!("Synced {} days, {} total entries", total_days, total_entries);

    Ok(())
}

#[cfg(test)]
mod tests;
