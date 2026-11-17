//! Runs the built `dialectic` binary the way an agent would.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn dialectic(repository_dir: &Path, arguments: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dialectic"))
        .arg("-C")
        .arg(repository_dir)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn stdout_of(output: &Output) -> String {
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_agent_creates_lists_edits_and_closes_a_pull_request() {
    let temporary_dir = tempfile::tempdir().unwrap();
    let repository_dir = temporary_dir.path();
    common::create_demo_repository(repository_dir);
    common::git(repository_dir, &["switch", "-q", "feature"]);

    let created = stdout_of(&dialectic(
        repository_dir,
        &["pr", "create", "--target", "main", "--title", "Add greeting", "--description-file", "-", "--json"],
        "- [x] says hello\n",
    ));
    let created: serde_json::Value = serde_json::from_str(&created).unwrap();
    assert_eq!(created["source_branch"], "feature", "--source defaults to the current branch");
    assert_eq!(created["description"], "- [x] says hello\n");
    let pull_request_id = created["id"].as_i64().unwrap().to_string();

    let shown: serde_json::Value =
        serde_json::from_str(&stdout_of(&dialectic(repository_dir, &["pr", "show", &pull_request_id, "--json"], "")))
            .unwrap();
    assert_eq!(shown["commits"].as_array().unwrap().len(), 2);
    assert_eq!(shown["changed_files"][0]["path"], "app.txt");
    assert!(stdout_of(&dialectic(repository_dir, &["pr", "diff", &pull_request_id], "")).contains("+hello"));

    stdout_of(&dialectic(repository_dir, &["pr", "edit", &pull_request_id, "--title", "Add a greeting"], ""));
    let listed = stdout_of(&dialectic(repository_dir, &["pr", "list", "--status", "open"], ""));
    assert!(listed.contains("Add a greeting") && listed.contains("feature → main"), "{listed}");

    stdout_of(&dialectic(repository_dir, &["pr", "close", &pull_request_id], ""));
    assert_eq!(stdout_of(&dialectic(repository_dir, &["pr", "list", "--status", "open"], "")), "No pull requests.\n");
}

#[test]
fn failures_explain_themselves_with_distinct_exit_codes() {
    let temporary_dir = tempfile::tempdir().unwrap();
    common::create_demo_repository(temporary_dir.path());

    let same_branch = dialectic(temporary_dir.path(), &["pr", "create", "--target", "main", "--title", "x"], "");
    assert_eq!(same_branch.status.code(), Some(1), "main is checked out, so source = target");
    assert!(String::from_utf8_lossy(&same_branch.stderr).contains("two different branches"));

    let usage = dialectic(temporary_dir.path(), &["pr", "create", "--title", "missing target"], "");
    assert_eq!(usage.status.code(), Some(2));

    let not_a_repository = dialectic(&temporary_dir.path().join(".git"), &["pr", "list"], "");
    assert_eq!(not_a_repository.status.code(), Some(1));

    let guide = stdout_of(&dialectic(temporary_dir.path(), &["guide"], ""));
    assert!(guide.contains("You cannot merge") && guide.contains("dialectic pr create"));
}
