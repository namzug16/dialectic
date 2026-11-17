mod common;

use dialectic::db::PullRequestStatus;
use dialectic::repository::Repository;

fn open_demo_repository() -> (tempfile::TempDir, Repository) {
    let temporary_dir = tempfile::tempdir().unwrap();
    common::create_demo_repository(temporary_dir.path());
    let repository = Repository::open(temporary_dir.path()).unwrap();
    (temporary_dir, repository)
}

#[test]
fn a_pull_request_lists_the_source_commits_and_the_merge_base_diff() {
    let (_temporary_dir, repository) = open_demo_repository();
    let pull_request_id = repository
        .create_pull_request("  Add greeting ", "Says **hello**", "feature", "main")
        .unwrap();
    let pull_request = repository.pull_request(pull_request_id).unwrap();
    assert_eq!(pull_request.title, "Add greeting");
    assert_eq!(pull_request.author, "Grace");
    assert_eq!(pull_request.status, PullRequestStatus::Open);

    let commit_subjects: Vec<String> = repository
        .commits(&pull_request)
        .unwrap()
        .into_iter()
        .map(|commit| commit.subject)
        .collect();
    assert_eq!(commit_subjects, vec!["add greeting", "tweak six"]);

    let diff = repository.diff(&pull_request).unwrap();
    assert!(
        diff.contains("+++ b/greeting.txt") && diff.contains("+six (feature)"),
        "{diff}"
    );
    assert!(
        !diff.contains("one (main)"),
        "main's own changes are not part of the pull request: {diff}"
    );
}

#[test]
fn creating_a_pull_request_enforces_its_rules() {
    let (temporary_dir, repository) = open_demo_repository();
    common::git(temporary_dir.path(), &["branch", "behind-main", "main~1"]);
    let rejection = |title: &str, source: &str, target: &str| {
        repository.create_pull_request(title, "", source, target).unwrap_err().0
    };
    assert!(rejection(" ", "feature", "main").contains("title"));
    assert!(rejection("t", "main", "main").contains("two different branches"));
    assert!(rejection("t", "nope", "main").contains("not a local branch"));
    assert!(rejection("t", "behind-main", "main").contains("no commits"));

    repository.create_pull_request("t", "", "feature", "main").unwrap();
    assert!(rejection("again", "feature", "main").contains("already open"));
}

#[test]
fn only_open_pull_requests_can_be_edited() {
    let (_temporary_dir, repository) = open_demo_repository();
    let pull_request_id = repository
        .create_pull_request("Old", "old body", "feature", "main")
        .unwrap();
    repository
        .edit_pull_request(pull_request_id, "New", "new body")
        .unwrap();
    let edited = repository.pull_request(pull_request_id).unwrap();
    assert_eq!(
        (edited.title.as_str(), edited.description.as_str()),
        ("New", "new body")
    );
    assert!(repository.edit_pull_request(pull_request_id, "", "x").is_err());
}
