//! "Your changes": the opened checkout's branch and uncommitted changes, with commit and branch switching,
//! so the whole way from editing files to opening a pull request stays in the app.

use super::pull_request_detail::files_changed;
use super::{Dialectic, error_banner};
use rune::{El, TextEdit, Tx, col, row, text};
use rune_kit::diff::DiffState;
use rune_kit::{Button, ButtonVariant, Kit, Select, TextInput};

#[derive(Default)]
pub struct WorkingChanges {
    pub local_branches: Vec<String>,
    /// `None` when HEAD is detached.
    pub current_branch: Option<String>,
    pub branch_picker_open: bool,
    pub diff: DiffState,
    pub commit_message: TextEdit,
    pub new_branch_name: TextEdit,
}

pub fn view(app: &Dialectic, k: &Kit, overlay_layer: &mut Vec<El>) -> El {
    let changes = &app.working_changes;
    let changed_file_count = changes.diff.files().len();
    let on_default_branch = matches!(changes.current_branch.as_deref(), Some("main" | "master"));

    let title_row = row()
        .w_full()
        .items_center()
        .child(text("Your changes").font(k.ty.body_medium).color(k.c.text_strong).grow(1.0))
        .child(
            Button::new("Refresh")
                .variant(ButtonVariant::Ghost)
                .help("Read the files again after editing them")
                .on_press(|app: &mut Dialectic| app.reload_changes())
                .build(k)
                .key("refresh-changes-button"),
        );

    let branch_picker = Select::new("current-branch", &changes.local_branches)
        .label("Current branch")
        .placeholder("No branch (detached)")
        .current(changes.current_branch.as_deref())
        .open(changes.branch_picker_open)
        .on_toggle(|app: &mut Dialectic| app.working_changes.branch_picker_open ^= true)
        .on_close(|app: &mut Dialectic| app.working_changes.branch_picker_open = false)
        .on_choose(|app: &mut Dialectic, chosen_branch: &str, _: &mut Tx| app.switch_branch(chosen_branch))
        .build(k, overlay_layer);
    let branch_row = row()
        .key("branch-row")
        .w_full()
        .padding(12.0)
        .gap(8.0)
        .items_center()
        .radius(k.radius.md)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_weak)
        .child(text("On branch").font(k.ty.ui).color(k.c.text_muted))
        .child(branch_picker)
        .child(text("or start a new one:").font(k.ty.ui).color(k.c.text_muted))
        .child(
            TextInput::new(&changes.new_branch_name)
                .label("New branch name")
                .placeholder("my-new-branch")
                .width(Some(200.0))
                .on_edit(|app: &mut Dialectic| &mut app.working_changes.new_branch_name)
                .on_submit(|app: &mut Dialectic, _: &mut Tx| app.create_branch())
                .build(k),
        )
        .child(
            Button::new("Create branch")
                .variant(ButtonVariant::Outline)
                .help("Your uncommitted changes come along to the new branch")
                .on_press(|app: &mut Dialectic| app.create_branch())
                .build(k)
                .key("create-branch-button"),
        );

    let mut content = col().w_full().max_w(1012.0).gap(12.0).child(title_row).child(branch_row);
    if on_default_branch && changed_file_count > 0 {
        content = content.child(
            row()
                .key("default-branch-warning")
                .w_full()
                .padding(12.0)
                .radius(k.radius.md)
                .fill(k.c.warning.bg)
                .border(1.0, k.c.warning.border)
                .child(
                    text(
                        "You're on the main branch. Start a new branch for this work first, so it can be reviewed \
                         in a pull request before it reaches main.",
                    )
                    .font(k.ty.ui)
                    .color(k.c.warning.fg)
                    .wrap(),
                ),
        );
    }
    if let Some(banner) = error_banner(app, k) {
        content = content.child(banner);
    }
    content = content.child(if changed_file_count == 0 {
        nothing_to_commit(changes, on_default_branch, k)
    } else {
        commit_row(changes, changed_file_count, k)
    });

    let top = col()
        .w_full()
        .px(24.0)
        .py(16.0)
        .items_center()
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(content);
    let mut screen = col().w_full().grow(1.0).child(top);
    if changed_file_count > 0 {
        screen = screen.child(files_changed(
            app,
            |app: &Dialectic| &app.working_changes.diff,
            |app: &mut Dialectic| &mut app.working_changes.diff,
            &[],
            k,
        ));
    }
    screen
}

/// Commits everything: a person new to git shouldn't have to learn the staging area first.
fn commit_row(changes: &WorkingChanges, changed_file_count: usize, k: &Kit) -> El {
    let files = if changed_file_count == 1 { "1 file".to_owned() } else { format!("{changed_file_count} files") };
    row()
        .key("commit-row")
        .w_full()
        .gap(8.0)
        .items_center()
        .child(
            col().grow(1.0).child(
                TextInput::new(&changes.commit_message)
                    .label("Commit message")
                    .placeholder("Describe what you changed, e.g. Fix the typo on the sign-up page")
                    .width(None)
                    .on_edit(|app: &mut Dialectic| &mut app.working_changes.commit_message)
                    .on_submit(|app: &mut Dialectic, _: &mut Tx| app.commit_all_changes())
                    .build(k),
            ),
        )
        .child(
            Button::new(format!("Commit {files}"))
                .variant(ButtonVariant::Submit)
                .on_press(|app: &mut Dialectic| app.commit_all_changes())
                .build(k)
                .key("commit-button"),
        )
}

/// After committing on a work branch, the next step is a pull request: offer it right here.
fn nothing_to_commit(changes: &WorkingChanges, on_default_branch: bool, k: &Kit) -> El {
    let mut empty_state = row()
        .key("no-changes")
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .items_center()
        .radius(k.radius.md)
        .border(1.0, k.c.border_weak)
        .child(text("No uncommitted changes.").font(k.ty.ui).color(k.c.text_muted).grow(1.0));
    if !on_default_branch && changes.current_branch.is_some() {
        empty_state = empty_state.child(
            Button::new("Open a pull request")
                .variant(ButtonVariant::Submit)
                .on_press(|app: &mut Dialectic| app.show_new_pull_request())
                .build(k)
                .key("open-pull-request-from-changes"),
        );
    }
    empty_state
}
