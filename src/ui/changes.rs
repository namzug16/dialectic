//! "Your changes": the opened checkout's branch and uncommitted changes, with commit and branch switching,
//! so the whole way from editing files to opening a pull request stays in the app.

use super::pull_request_detail::files_changed;
use super::{Dialectic, Strings, error_banner, icons, pull_requests_button};
use rune::{El, TextEdit, Tx, col, icon, row, text};
use rune_kit::diff::DiffState;
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Kit, Select, TextInput};

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
    let s = app.strings();
    let changed_file_count = changes.diff.files().len();
    let on_default_branch = matches!(changes.current_branch.as_deref(), Some("main" | "master"));

    let branch_picker = Select::new("current-branch", &changes.local_branches)
        .label(s.current_branch)
        .placeholder(s.detached_head)
        .current(changes.current_branch.as_deref())
        .open(changes.branch_picker_open)
        .on_toggle(|app: &mut Dialectic| app.working_changes.branch_picker_open ^= true)
        .on_close(|app: &mut Dialectic| app.working_changes.branch_picker_open = false)
        .on_choose(|app: &mut Dialectic, chosen_branch: &str, _: &mut Tx| app.switch_branch(chosen_branch))
        .build(k, overlay_layer);
    // One bar: the branch and a new one on the left, the commit on the right, so the diff keeps the window.
    let mut toolbar = row()
        .key("branch-row")
        .w_full()
        .px(16.0)
        .py(8.0)
        .gap(8.0)
        .items_center()
        .fill(k.c.layer_01)
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(icon(&icons::GIT_BRANCH, 16.0).color(k.c.text_muted))
        .child(branch_picker)
        .child(
            TextInput::new(&changes.new_branch_name)
                .label(s.new_branch_name)
                .placeholder(s.new_branch_placeholder)
                .width(Some(180.0))
                .on_edit(|app: &mut Dialectic| &mut app.working_changes.new_branch_name)
                .on_submit(|app: &mut Dialectic, _: &mut Tx| app.create_branch())
                .build(k),
        )
        .child(
            IconButton::new(&icons::GIT_BRANCH_PLUS)
                .variant(IconButtonVariant::Ghost)
                .label(s.create_branch)
                .help(s.create_branch_help)
                .on_press(|app: &mut Dialectic| app.create_branch())
                .build(k)
                .key("create-branch-button"),
        )
        .child(col().w(1.0).h(20.0).mx(4.0).fill(k.c.border_weak));
    toolbar = if changed_file_count > 0 {
        toolbar.child(commit_message_input(changes, s, k)).child(
            Button::new((s.commit_files)(changed_file_count))
                .variant(ButtonVariant::Submit)
                .icon(&icons::GIT_COMMIT)
                .on_press(|app: &mut Dialectic| app.commit_all_changes())
                .build(k)
                .key("commit-button"),
        )
    } else {
        toolbar.child(col().grow(1.0))
    };
    toolbar = toolbar.child(
        IconButton::new(&icons::REFRESH)
            .variant(IconButtonVariant::Ghost)
            .label(s.refresh)
            .help(s.refresh_help)
            .on_press(|app: &mut Dialectic| app.reload_changes())
            .build(k)
            .key("refresh-changes-button"),
    );

    let mut top = col().w_full().child(toolbar);
    if on_default_branch && changed_file_count > 0 {
        top = top.child(notice(
            "default-branch-warning",
            text(s.default_branch_warning).font(k.ty.ui).color(k.c.warning.fg).wrap(),
            k.c.warning.bg,
            k.c.warning.border,
        ));
    }
    if let Some(banner) = error_banner(app, k) {
        top = top.child(col().w_full().px(16.0).py(8.0).child(banner));
    }
    let body = if changed_file_count == 0 {
        nothing_to_commit(changes, on_default_branch, app.open_pull_request_count(), s, k)
    } else {
        files_changed(
            app,
            |app: &Dialectic| &app.working_changes.diff,
            |app: &mut Dialectic| &mut app.working_changes.diff,
            &[],
            s,
            k,
        )
    };
    col().w_full().grow(1.0).min_h(0.0).child(top).child(body)
}

/// Commits everything: a person new to git shouldn't have to learn the staging area first.
fn commit_message_input(changes: &WorkingChanges, s: &Strings, k: &Kit) -> El {
    col().grow(1.0).min_w(160.0).child(
        TextInput::new(&changes.commit_message)
            .label(s.commit_message)
            .placeholder(s.commit_message_placeholder)
            .width(None)
            .on_edit(|app: &mut Dialectic| &mut app.working_changes.commit_message)
            .on_submit(|app: &mut Dialectic, _: &mut Tx| app.commit_all_changes())
            .build(k),
    )
}

/// A full-width strip under the bar, in a status colour.
fn notice(key: &str, message: El, fill: rune::Color, border: rune::Color) -> El {
    row()
        .key(key.to_owned())
        .w_full()
        .px(16.0)
        .py(8.0)
        .fill(fill)
        .border_sides([0.0, 0.0, 1.0, 0.0], border)
        .child(message)
}

/// Centred in the empty space: says there's nothing to commit, then points at what to do instead,
/// loudest when pull requests are waiting for review.
fn nothing_to_commit(
    changes: &WorkingChanges,
    on_default_branch: bool,
    open_pull_request_count: usize,
    s: &Strings,
    k: &Kit,
) -> El {
    let mut empty_state = col()
        .key("no-changes")
        .w_full()
        .max_w(560.0)
        .gap(16.0)
        .items_center()
        .child(text(s.no_uncommitted_changes).font(k.ty.body_medium).color(k.c.text_strong))
        .child(text(s.no_uncommitted_changes_hint).font(k.ty.ui).color(k.c.text_muted).wrap());
    if open_pull_request_count > 0 {
        let waiting = (s.pull_requests_waiting)(open_pull_request_count);
        empty_state = empty_state.child(
            col()
                .key("pull-requests-waiting")
                .w_full()
                .padding(20.0)
                .gap(12.0)
                .items_center()
                .radius(k.radius.md)
                .fill(k.c.success.bg)
                .border(2.0, k.c.success.border)
                .child(text(waiting).font(k.ty.body_medium).color(k.c.success.fg))
                .child(pull_requests_button(open_pull_request_count, s, k)),
        );
    }
    // After committing on a work branch, the next step is a pull request: offer it right here.
    if !on_default_branch && changes.current_branch.is_some() {
        empty_state = empty_state.child(
            Button::new(s.open_a_pull_request)
                .icon(&icons::GIT_PULL_REQUEST)
                .variant(if open_pull_request_count > 0 { ButtonVariant::Outline } else { ButtonVariant::Submit })
                .on_press(|app: &mut Dialectic| app.show_new_pull_request())
                .build(k)
                .key("open-pull-request-from-changes"),
        );
    }
    col().w_full().grow(1.0).padding(24.0).center().child(empty_state)
}
