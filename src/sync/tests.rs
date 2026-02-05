use super::{compute_allocations, is_weekday};
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use std::collections::HashMap;

use crate::parser;

fn session(project: &str, duration_secs: i64) -> parser::Session {
    parser::Session {
        start: "2026-02-04T10:00:00Z".parse().unwrap(),
        end: "2026-02-04T10:00:00Z".parse().unwrap(),
        duration: TimeDelta::seconds(duration_secs),
        project: project.to_string(),
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    }
}

fn utc(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

fn mapping(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// Work day: 09:00–17:00 UTC on 2026-02-04 (8h = 28800s).
// All tests call compute_allocations directly to avoid local→UTC timezone dependency.
const START: &str = "2026-02-04T09:00:00Z";
const END: &str = "2026-02-04T17:00:00Z";
const WORK_DAY_SECS: i64 = 28800;

#[test]
fn zero_sessions_returns_empty() {
    let result = compute_allocations(
        &[],
        &mapping(&[("/work/foo", "proj-foo")]),
        &Some("proj-other".into()),
        utc(START),
        utc(END),
    );
    assert!(result.allocations.is_empty());
    assert!(result.skipped.is_empty());
}

#[test]
fn single_mapped_project_fills_entire_work_day() {
    let sessions = vec![session("/work/myapp", 3600)];
    let result = compute_allocations(
        &sessions,
        &mapping(&[("/work/myapp", "proj-myapp")]),
        &Some("proj-other".into()),
        utc(START),
        utc(END),
    );

    assert_eq!(result.allocations.len(), 1);
    assert_eq!(result.allocations[0].project_id, "proj-myapp");
    assert_eq!(result.allocations[0].start, utc(START));
    assert_eq!(result.allocations[0].end, utc(END));
    assert!(result.skipped.is_empty());
}

#[test]
fn two_projects_split_proportionally() {
    // 3h + 1h tracked → 3:1 ratio → 6h + 2h on 8h work day.
    let sessions = vec![
        session("/work/alpha", 10800), // 3h
        session("/work/beta", 3600),   // 1h
    ];

    let result = compute_allocations(
        &sessions,
        &mapping(&[("/work/alpha", "proj-a"), ("/work/beta", "proj-b")]),
        &Some("proj-other".into()),
        utc(START),
        utc(END),
    );

    assert_eq!(result.allocations.len(), 2);
    // Sorted by project_id: proj-a, proj-b.
    assert_eq!(result.allocations[0].project_id, "proj-a");
    assert_eq!(result.allocations[0].start, utc(START));
    assert_eq!(result.allocations[0].end, utc("2026-02-04T15:00:00Z")); // 09:00 + 6h
    assert_eq!(result.allocations[1].project_id, "proj-b");
    assert_eq!(result.allocations[1].start, utc("2026-02-04T15:00:00Z"));
    assert_eq!(result.allocations[1].end, utc(END)); // 15:00 + 2h
    assert!(result.skipped.is_empty());
}

#[test]
fn unmapped_projects_aggregate_into_other() {
    // 2h mapped + 2h unmapped, Other enabled → 1:1 ratio → 4h each.
    let sessions = vec![
        session("/work/mapped", 7200),  // 2h
        session("/work/unknown", 7200), // 2h, unmapped
    ];

    let result = compute_allocations(
        &sessions,
        &mapping(&[("/work/mapped", "proj-mapped")]),
        &Some("proj-other".into()),
        utc(START),
        utc(END),
    );

    assert_eq!(result.allocations.len(), 2);
    // proj-mapped < proj-other alphabetically.
    assert_eq!(result.allocations[0].project_id, "proj-mapped");
    assert_eq!(result.allocations[0].start, utc(START));
    assert_eq!(result.allocations[0].end, utc("2026-02-04T13:00:00Z")); // 09:00 + 4h

    assert_eq!(result.allocations[1].project_id, "proj-other");
    assert_eq!(result.allocations[1].start, utc("2026-02-04T13:00:00Z"));
    assert_eq!(result.allocations[1].end, utc(END)); // 13:00 + 4h

    assert!(result.skipped.is_empty());
}

#[test]
fn unmapped_projects_skipped_when_other_disabled() {
    // 2h mapped + 2h unmapped, Other disabled → mapped gets full 8h, unmapped skipped.
    let sessions = vec![
        session("/work/mapped", 7200),  // 2h
        session("/work/unknown", 7200), // 2h, unmapped → skipped
    ];

    let result = compute_allocations(
        &sessions,
        &mapping(&[("/work/mapped", "proj-mapped")]),
        &None,
        utc(START),
        utc(END),
    );

    assert_eq!(result.allocations.len(), 1);
    assert_eq!(result.allocations[0].project_id, "proj-mapped");
    assert_eq!(result.allocations[0].start, utc(START));
    assert_eq!(result.allocations[0].end, utc(END));
    assert_eq!(result.skipped, vec!["/work/unknown"]);
}

#[test]
fn no_other_entry_when_all_projects_mapped() {
    let sessions = vec![session("/work/alpha", 3600), session("/work/beta", 3600)];

    let result = compute_allocations(
        &sessions,
        &mapping(&[("/work/alpha", "proj-a"), ("/work/beta", "proj-b")]),
        &Some("proj-other".into()),
        utc(START),
        utc(END),
    );
    assert!(result
        .allocations
        .iter()
        .all(|a| a.project_id != "proj-other"));
    assert!(result.skipped.is_empty());
}

#[test]
fn allocations_are_contiguous_from_work_day_start() {
    let sessions = vec![
        session("/work/alpha", 5000),
        session("/work/beta", 3000),
        session("/work/gamma", 2000),
    ];

    let result = compute_allocations(
        &sessions,
        &mapping(&[
            ("/work/alpha", "proj-a"),
            ("/work/beta", "proj-b"),
            ("/work/gamma", "proj-c"),
        ]),
        &None,
        utc(START),
        utc(END),
    );

    assert_eq!(result.allocations.len(), 3);
    assert_eq!(result.allocations[0].start, utc(START));
    for i in 1..result.allocations.len() {
        assert_eq!(
            result.allocations[i].start,
            result.allocations[i - 1].end,
            "gap between allocation {} and {}",
            i - 1,
            i
        );
    }
    assert_eq!(result.allocations.last().unwrap().end, utc(END));
}

#[test]
fn last_entry_absorbs_rounding_remainder() {
    // Durations 3000:3000:1000 (ratio 3:3:1 over 7).
    // 28800 * 3/7 = 12342.857... → floor 12342 each for proj-a, proj-b.
    // 28800 * 1/7 = 4114.285...  → floor 4114 for proj-c.
    // Sum of floors = 28798. Remainder 2 → proj-c (last, alphabetically) = 4116.
    let sessions = vec![
        session("/work/alpha", 3000),
        session("/work/beta", 3000),
        session("/work/gamma", 1000),
    ];

    let result = compute_allocations(
        &sessions,
        &mapping(&[
            ("/work/alpha", "proj-a"),
            ("/work/beta", "proj-b"),
            ("/work/gamma", "proj-c"),
        ]),
        &None,
        utc(START),
        utc(END),
    );

    let durations: Vec<i64> = result
        .allocations
        .iter()
        .map(|a| (a.end - a.start).num_seconds())
        .collect();

    assert_eq!(durations, vec![12342, 12342, 4116]);
    assert_eq!(durations.iter().sum::<i64>(), WORK_DAY_SECS);
}

// --- Sync loop tests ---

#[test]
fn is_weekday_filters_correctly() {
    // 2026-02-02 is Monday
    // 2026-02-07 is Saturday
    // 2026-02-08 is Sunday
    let mon = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();
    let tue = NaiveDate::from_ymd_opt(2026, 2, 3).unwrap();
    let wed = NaiveDate::from_ymd_opt(2026, 2, 4).unwrap();
    let thu = NaiveDate::from_ymd_opt(2026, 2, 5).unwrap();
    let fri = NaiveDate::from_ymd_opt(2026, 2, 6).unwrap();
    let sat = NaiveDate::from_ymd_opt(2026, 2, 7).unwrap();
    let sun = NaiveDate::from_ymd_opt(2026, 2, 8).unwrap();

    assert!(is_weekday(mon), "Monday should be a weekday");
    assert!(is_weekday(tue), "Tuesday should be a weekday");
    assert!(is_weekday(wed), "Wednesday should be a weekday");
    assert!(is_weekday(thu), "Thursday should be a weekday");
    assert!(is_weekday(fri), "Friday should be a weekday");
    assert!(!is_weekday(sat), "Saturday should not be a weekday");
    assert!(!is_weekday(sun), "Sunday should not be a weekday");
}

/// Integration test: run_sync processes multiple days end-to-end
/// This test hits the real Clockify API and requires:
/// - Clockify API key in keyring
/// - Valid sync config in config.toml
/// - Test sessions in the database
/// Run with: cargo test -- --ignored
#[test]
#[ignore]
fn test_run_sync_multiple_days() {
    // TODO:
    // 1. Set up test sessions in a temp database for 2-3 workdays
    // 2. Call run_sync()
    // 3. Verify entries were created in Clockify (check UI or query API)
    // 4. Verify idempotency: run_sync again, no duplicates
    // 5. Clean up: delete test entries from Clockify
    assert!(true == false, "Scaffold: implement integration test");
}
