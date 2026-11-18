//! Clicks through the GUI headless (Rune's `Runtime`, same layout and hit testing as the window).

mod common;

use dialectic::db::PullRequestStatus;
use dialectic::ui::{Dialectic, Language, Screen, view};
use rune::{Env, Runtime, TextEdit};

fn press(runtime: &mut Runtime<Dialectic>, key: &str) {
    let (x, y) = runtime.find(key).unwrap_or_else(|| panic!("no element keyed {key:?} on screen"));
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    settle(runtime);
}

/// Half a second of frames, so the next screen has mounted and finished fading in.
fn settle(runtime: &mut Runtime<Dialectic>) {
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
}

#[test]
fn open_review_and_merge_a_pull_request_then_hit_a_conflict() {
    let temporary_dir = tempfile::tempdir().unwrap();
    let repository_dir = temporary_dir.path().join("repository");
    std::fs::create_dir(&repository_dir).unwrap();
    common::create_demo_repository(&repository_dir);
    // SAFETY: the only test in this binary, set before any thread reads the environment.
    unsafe { std::env::set_var("DIALECTIC_DATA_DIR", temporary_dir.path().join("app-data")) };

    let mut app = Dialectic::without_repository();
    app.open_repository(&repository_dir);
    let mut runtime = Runtime::new(app, view, (1100.0, 760.0), Env { system_dark: false, scale: 2.0 });
    runtime.frame(0.0);
    assert_eq!(runtime.state.screen, Screen::Changes, "a repository opens on your changes");
    assert!(runtime.find("no-changes").is_some() && runtime.find("pull-requests-waiting").is_none());
    press(&mut runtime, "show-pull-requests-button");
    assert_eq!(runtime.state.screen, Screen::PullRequestList);

    press(&mut runtime, "new-pull-request-button");
    assert_eq!(runtime.state.screen, Screen::NewPullRequest);
    runtime.update(|app, _| {
        app.new_pull_request_form.source_branch = Some("feature".into());
        app.draft.title = TextEdit::new("Add greeting");
        app.draft.description = TextEdit::new("- [x] greet");
    });
    press(&mut runtime, "create-button");
    assert_eq!(runtime.state.screen, Screen::PullRequestDetail, "{:?}", runtime.state.error_message);
    assert_eq!(runtime.state.pull_request_diff.files().len(), 2);

    press(&mut runtime, "edit-button");
    runtime.update(|app, _| app.draft.title = TextEdit::new("Add a greeting"));
    press(&mut runtime, "save-edit-button");
    let detail = runtime.state.pull_request_detail.as_ref().unwrap();
    assert_eq!(detail.pull_request.title, "Add a greeting");

    press(&mut runtime, "merge-button");
    let detail = runtime.state.pull_request_detail.as_ref().unwrap();
    assert_eq!(detail.pull_request.status, PullRequestStatus::Merged, "{:?}", runtime.state.error_message);
    assert!(common::git(&repository_dir, &["ls-tree", "--name-only", "main"]).contains("greeting.txt"));

    // Typing isn't what's under test: set the form as a person would leave it.
    runtime.update(|app, _| {
        app.show_new_pull_request();
        app.new_pull_request_form.source_branch = Some("conflict".into());
        app.draft.title = TextEdit::new("Change one");
    });
    settle(&mut runtime);
    press(&mut runtime, "create-button");
    let detail = runtime.state.pull_request_detail.as_ref().unwrap();
    assert_eq!(detail.merge_conflicts, vec!["app.txt".to_owned()], "predicted before anyone merges");
    assert!(runtime.find("conflict-banner").is_some() && runtime.find("conflicted-files").is_some());
    assert!(runtime.find("agent-prompt").is_some(), "suggests asking the owner or a coding agent");
    assert!(runtime.find("merge-button").is_none() && runtime.find("merge-method").is_none(), "merging is blocked");
    press(&mut runtime, "detail-tab-2");
    assert_eq!(runtime.state.pull_request_diff.files()[0].path(), "app.txt", "conflicting files come first");

    press(&mut runtime, "detail-tab-0");
    press(&mut runtime, "close-button");
    assert_eq!(runtime.state.pull_request_detail.as_ref().unwrap().pull_request.status, PullRequestStatus::Closed);

    // Your changes: branch off main, commit everything, then open a pull request from it.
    runtime.update(|app, _| {
        app.repository.as_ref().unwrap().create_pull_request("Change one again", "", "conflict", "main").unwrap();
    });
    press(&mut runtime, "back-to-pull-requests");
    press(&mut runtime, "show-changes-button");
    assert_eq!(runtime.state.screen, Screen::Changes);
    assert!(runtime.find("no-changes").is_some());
    assert!(runtime.find("pull-requests-waiting").is_some(), "an open pull request is called out");
    std::fs::write(repository_dir.join("notes.txt"), "a new file\n").unwrap();
    press(&mut runtime, "refresh-changes-button");
    assert_eq!(runtime.state.working_changes.diff.files().len(), 1);
    assert!(runtime.find("default-branch-warning").is_some(), "warns before committing straight to main");
    runtime.update(|app, _| app.working_changes.new_branch_name = TextEdit::new("notes"));
    press(&mut runtime, "create-branch-button");
    assert_eq!(
        runtime.state.working_changes.current_branch.as_deref(),
        Some("notes"),
        "{:?}",
        runtime.state.error_message
    );
    runtime.update(|app, _| app.working_changes.commit_message = TextEdit::new("Add notes"));
    press(&mut runtime, "commit-button");
    assert!(runtime.state.working_changes.diff.files().is_empty(), "{:?}", runtime.state.error_message);
    assert_eq!(common::git(&repository_dir, &["log", "-1", "--format=%s", "notes"]).trim(), "Add notes");
    press(&mut runtime, "open-pull-request-from-changes");
    assert_eq!(runtime.state.new_pull_request_form.source_branch.as_deref(), Some("notes"));

    runtime.update(|app, _| app.show_changes());
    settle(&mut runtime);
    runtime.update(|app, _| app.switch_branch("main"));
    assert_eq!(runtime.state.working_changes.current_branch.as_deref(), Some("main"));

    press(&mut runtime, "back-to-repositories");
    assert_eq!(runtime.state.screen, Screen::Welcome);

    assert!(runtime.find("language-picker").is_none(), "the language is set in Settings");
    press(&mut runtime, "settings-button");
    assert_eq!(runtime.state.screen, Screen::Settings);
    assert!(runtime.find("language-picker").is_some() && runtime.find("source-code-link").is_some());
    runtime.update(|app, _| app.set_language(Language::Spanish));
    press(&mut runtime, "back-to-repositories");
    assert_eq!(runtime.state.screen, Screen::Welcome);
    assert_eq!(Dialectic::without_repository().language, Language::Spanish, "the language is remembered");
}
