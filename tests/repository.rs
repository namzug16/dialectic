mod common;

use dialectic::db::{MergeMethod, PullRequestStatus};
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
    let pull_request_id =
        repository.create_pull_request("  Add greeting ", "Says **hello**", "feature", "main").unwrap();
    let pull_request = repository.pull_request(pull_request_id).unwrap();
    assert_eq!(pull_request.title, "Add greeting");
    assert_eq!(pull_request.author, "Grace");
    assert_eq!(pull_request.status, PullRequestStatus::Open);

    let commit_subjects: Vec<String> =
        repository.commits(&pull_request).unwrap().into_iter().map(|commit| commit.subject).collect();
    assert_eq!(commit_subjects, vec!["add greeting", "tweak six"]);

    let diff = repository.diff(&pull_request).unwrap();
    assert!(diff.contains("+++ b/greeting.txt") && diff.contains("+six (feature)"), "{diff}");
    assert!(!diff.contains("one (main)"), "main's own changes are not part of the pull request: {diff}");
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
    let pull_request_id = repository.create_pull_request("Old", "old body", "feature", "main").unwrap();
    repository.edit_pull_request(pull_request_id, "New", "new body").unwrap();
    let edited = repository.pull_request(pull_request_id).unwrap();
    assert_eq!((edited.title.as_str(), edited.description.as_str()), ("New", "new body"));
    assert!(repository.edit_pull_request(pull_request_id, "", "x").is_err());
}

use dialectic::repository::MergeOutcome;

#[test]
fn a_clean_merge_creates_a_merge_commit_and_keeps_the_pull_request_viewable() {
    let (temporary_dir, repository) = open_demo_repository();
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();
    let diff_before_merge = repository.diff(&repository.pull_request(pull_request_id).unwrap()).unwrap();

    assert_eq!(repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).unwrap(), MergeOutcome::Merged);

    let main_parents = common::git(temporary_dir.path(), &["log", "-1", "--format=%P", "main"]);
    assert_eq!(main_parents.split_whitespace().count(), 2, "--no-ff always makes a merge commit");
    assert!(temporary_dir.path().join("greeting.txt").exists(), "main's checkout was updated");

    let merged = repository.pull_request(pull_request_id).unwrap();
    assert_eq!(merged.status, PullRequestStatus::Merged);
    assert_eq!(repository.commits(&merged).unwrap().len(), 2);
    assert_eq!(repository.diff(&merged).unwrap(), diff_before_merge);
    assert!(
        repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).is_err(),
        "merging twice is refused"
    );
}

#[test]
fn a_conflicting_merge_reports_the_files_and_leaves_the_target_untouched() {
    let (temporary_dir, repository) = open_demo_repository();
    let main_before = common::git(temporary_dir.path(), &["rev-parse", "main"]);
    let pull_request_id = repository.create_pull_request("Change one", "", "conflict", "main").unwrap();

    let outcome = repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).unwrap();

    assert_eq!(outcome, MergeOutcome::Conflicts(vec!["app.txt".into()]));
    assert_eq!(common::git(temporary_dir.path(), &["rev-parse", "main"]), main_before);
    assert!(common::git(temporary_dir.path(), &["status", "--porcelain"]).is_empty(), "the merge was aborted");
    assert_eq!(repository.pull_request(pull_request_id).unwrap().status, PullRequestStatus::Open);
}

#[test]
fn a_target_not_checked_out_merges_in_a_temporary_worktree() {
    let (temporary_dir, repository) = open_demo_repository();
    common::git(temporary_dir.path(), &["switch", "-q", "conflict"]);
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();

    assert_eq!(repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).unwrap(), MergeOutcome::Merged);

    let main_files = common::git(temporary_dir.path(), &["ls-tree", "--name-only", "main"]);
    assert!(main_files.contains("greeting.txt"));
    assert_eq!(common::git(temporary_dir.path(), &["worktree", "list"]).lines().count(), 1, "removed afterwards");
    assert_eq!(common::git(temporary_dir.path(), &["branch", "--show-current"]).trim(), "conflict");
}

#[test]
fn a_dirty_target_checkout_is_never_merged_into() {
    let (temporary_dir, repository) = open_demo_repository();
    std::fs::write(temporary_dir.path().join("app.txt"), "uncommitted work\n").unwrap();
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();

    let refusal = repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).unwrap_err();

    assert!(refusal.0.contains("uncommitted changes"), "{refusal}");
    let unchanged = std::fs::read_to_string(temporary_dir.path().join("app.txt")).unwrap();
    assert_eq!(unchanged, "uncommitted work\n");
}

#[test]
fn a_closed_pull_request_cannot_be_edited_or_merged() {
    let (_temporary_dir, repository) = open_demo_repository();
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();
    repository.close_pull_request(pull_request_id).unwrap();

    let closed = repository.pull_request(pull_request_id).unwrap();
    assert_eq!(closed.status, PullRequestStatus::Closed);
    assert!(!repository.diff(&closed).unwrap().is_empty());
    assert!(repository.edit_pull_request(pull_request_id, "x", "y").is_err());
    assert!(repository.merge_pull_request(pull_request_id, MergeMethod::MergeCommit).is_err());
    repository.create_pull_request("Try again", "", "feature", "main").unwrap();
}

#[test]
fn rebase_and_merge_replays_the_commits_onto_the_target_without_a_merge_commit() {
    let (temporary_dir, repository) = open_demo_repository();
    let feature_before = common::git(temporary_dir.path(), &["rev-parse", "feature"]);
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();
    let diff_before_merge = repository.diff(&repository.pull_request(pull_request_id).unwrap()).unwrap();

    assert_eq!(repository.merge_pull_request(pull_request_id, MergeMethod::Rebase).unwrap(), MergeOutcome::Merged);

    let main_subjects = common::git(temporary_dir.path(), &["log", "--format=%s", "-3", "main"]);
    assert_eq!(main_subjects, "tweak six\nadd greeting\nchange one on main\n", "replayed on top of main");
    let main_parents = common::git(temporary_dir.path(), &["log", "--format=%P", "-3", "main"]);
    assert!(main_parents.lines().all(|parents| parents.split_whitespace().count() == 1), "no merge commit");
    assert!(temporary_dir.path().join("greeting.txt").exists(), "main's checkout fast-forwarded");
    assert_eq!(common::git(temporary_dir.path(), &["rev-parse", "feature"]), feature_before, "source never moves");
    assert_eq!(common::git(temporary_dir.path(), &["worktree", "list"]).lines().count(), 1);

    let merged = repository.pull_request(pull_request_id).unwrap();
    assert_eq!((merged.status, merged.merge_method), (PullRequestStatus::Merged, Some(MergeMethod::Rebase)));
    assert_eq!(repository.diff(&merged).unwrap(), diff_before_merge);
}

#[test]
fn a_conflicting_rebase_reports_the_files_and_leaves_everything_untouched() {
    let (temporary_dir, repository) = open_demo_repository();
    let main_before = common::git(temporary_dir.path(), &["rev-parse", "main"]);
    let pull_request_id = repository.create_pull_request("Change one", "", "conflict", "main").unwrap();

    let outcome = repository.merge_pull_request(pull_request_id, MergeMethod::Rebase).unwrap();

    assert_eq!(outcome, MergeOutcome::Conflicts(vec!["app.txt".into()]));
    assert_eq!(common::git(temporary_dir.path(), &["rev-parse", "main"]), main_before);
    assert!(common::git(temporary_dir.path(), &["status", "--porcelain"]).is_empty());
    assert_eq!(
        common::git(temporary_dir.path(), &["worktree", "list"]).lines().count(),
        1,
        "temporary worktree removed"
    );
    assert_eq!(repository.pull_request(pull_request_id).unwrap().status, PullRequestStatus::Open);
}

#[test]
fn rebasing_into_a_target_nobody_has_checked_out_moves_only_the_branch() {
    let (temporary_dir, repository) = open_demo_repository();
    common::git(temporary_dir.path(), &["switch", "-q", "conflict"]);
    let pull_request_id = repository.create_pull_request("Add greeting", "", "feature", "main").unwrap();

    assert_eq!(repository.merge_pull_request(pull_request_id, MergeMethod::Rebase).unwrap(), MergeOutcome::Merged);

    assert!(common::git(temporary_dir.path(), &["ls-tree", "--name-only", "main"]).contains("greeting.txt"));
    assert_eq!(common::git(temporary_dir.path(), &["branch", "--show-current"]).trim(), "conflict");
    assert!(common::git(temporary_dir.path(), &["status", "--porcelain"]).is_empty());
}
