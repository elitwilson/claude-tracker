use super::*;
use std::fs;

#[test]
fn finds_session_files() {
    let dir = tempfile::tempdir().unwrap();

    let proj1 = dir.path().join("-Users-foo-project1");
    fs::create_dir(&proj1).unwrap();
    fs::File::create(proj1.join("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl")).unwrap();
    fs::File::create(proj1.join("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb.jsonl")).unwrap();

    let proj2 = dir.path().join("-Users-foo-project2");
    fs::create_dir(&proj2).unwrap();
    fs::File::create(proj2.join("cccccccc-cccc-cccc-cccc-cccccccccccc.jsonl")).unwrap();

    let results = find_session_files(dir.path());

    assert_eq!(results.len(), 3);
}

#[test]
fn skips_agent_files() {
    let dir = tempfile::tempdir().unwrap();

    let proj = dir.path().join("-Users-foo-project1");
    fs::create_dir(&proj).unwrap();
    fs::File::create(proj.join("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl")).unwrap();
    fs::File::create(proj.join("agent-a095737.jsonl")).unwrap();

    let results = find_session_files(dir.path());

    assert_eq!(results.len(), 1);
}

#[test]
fn skips_subdirectories() {
    let dir = tempfile::tempdir().unwrap();

    let proj = dir.path().join("-Users-foo-project1");
    fs::create_dir(&proj).unwrap();
    fs::File::create(proj.join("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl")).unwrap();
    fs::create_dir(proj.join("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")).unwrap();

    let results = find_session_files(dir.path());

    assert_eq!(results.len(), 1);
}
