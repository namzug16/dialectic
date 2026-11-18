//! One pull request: GitHub's header and its Overview, Commits and Files changed tabs.

use super::{Dialectic, Strings, branch_flow, error_banner, markdown_editor, status_badge, tab_strip, warning_mark};
use crate::db::{MergeMethod, PullRequest, PullRequestStatus};
use crate::git::CommitSummary;
use rune::{El, Tx, col, row, text};
use rune_kit::diff::{DiffMode, DiffState, DiffView};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl};

pub const OVERVIEW_TAB: usize = 0;
pub const COMMITS_TAB: usize = 1;
pub const FILES_CHANGED_TAB: usize = 2;

pub struct PullRequestDetail {
    pub pull_request: PullRequest,
    pub commits: Vec<CommitSummary>,
    pub selected_tab: usize,
    /// Files that conflict between the branches: predicted when the pull request loads, or reported by a
    /// merge attempt (which git aborted). Any at all hides the merge buttons.
    pub merge_conflicts: Vec<String>,
    /// The title and description are in `Dialectic::draft`, being edited.
    pub is_editing: bool,
}

pub fn view(app: &Dialectic, detail: &PullRequestDetail, k: &Kit) -> El {
    let pull_request = &detail.pull_request;
    let s = app.strings();
    let changed_file_count = app.pull_request_diff.files().len();
    let tabs = tab_strip(
        k,
        "detail-tab",
        &[
            s.overview_tab.to_owned(),
            (s.commits_tab)(detail.commits.len()),
            (s.files_changed_tab)(changed_file_count, detail.merge_conflicts.len()),
        ]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>(),
        detail.selected_tab,
        |app: &mut Dialectic, picked_tab| {
            if let Some(detail) = &mut app.pull_request_detail {
                detail.selected_tab = picked_tab;
            }
        },
    );

    let mut summary = col()
        .w_full()
        .max_w(1012.0)
        .gap(8.0)
        .child(
            row()
                .w_full()
                .gap(8.0)
                .items_center()
                .child(text(pull_request.title.clone()).font(title_font(k)).color(k.c.text_strong))
                .child(text(format!("#{}", pull_request.id)).font(title_font(k)).color(k.c.text_faint).grow(1.0))
                .child(if pull_request.status == PullRequestStatus::Open && !detail.is_editing {
                    Button::new(s.edit)
                        .variant(ButtonVariant::Outline)
                        .on_press(|app: &mut Dialectic| app.start_editing_shown_pull_request())
                        .build(k)
                        .key("edit-button")
                } else {
                    col()
                }),
        )
        .child(
            row()
                .gap(8.0)
                .items_center()
                .child(status_badge(pull_request.status, s, k))
                .child(
                    text((s.activity_sentence)(pull_request.status, &pull_request.author, detail.commits.len()))
                        .font(k.ty.ui)
                        .color(k.c.text_muted),
                )
                .child(branch_flow(&pull_request.source_branch, &pull_request.target_branch, k)),
        );
    if !detail.merge_conflicts.is_empty() {
        summary = summary.child(conflict_banner(pull_request, detail.merge_conflicts.len(), s, k));
    }
    if let Some(banner) = error_banner(app, k) {
        summary = summary.child(banner);
    }

    let top = col()
        .w_full()
        .px(24.0)
        .pt(24.0)
        .gap(16.0)
        .items_center()
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(summary)
        .child(col().w_full().max_w(1012.0).child(tabs));

    let tab_body = match detail.selected_tab {
        FILES_CHANGED_TAB => files_changed(
            app,
            |app: &Dialectic| &app.pull_request_diff,
            |app: &mut Dialectic| &mut app.pull_request_diff,
            &detail.merge_conflicts,
            s,
            k,
        ),
        COMMITS_TAB => scrolling_column(commit_list(&detail.commits, s, k)),
        _ => scrolling_column(overview(app, detail, k)),
    };
    col().w_full().grow(1.0).child(top).child(tab_body)
}

/// Shown on every tab while the branches conflict, so a reviewer can't miss it.
fn conflict_banner(pull_request: &PullRequest, conflicted_file_count: usize, s: &Strings, k: &Kit) -> El {
    row()
        .key("conflict-banner")
        .w_full()
        .padding(12.0)
        .gap(10.0)
        .items_center()
        .radius(k.radius.md)
        .fill(k.c.danger.bg)
        .border(1.0, k.c.danger.border)
        .child(warning_mark(k))
        .child(
            text((s.conflict_banner)(&pull_request.source_branch, &pull_request.target_branch, conflicted_file_count))
                .font(k.ty.ui_medium)
                .color(k.c.danger.fg)
                .wrap(),
        )
}

fn title_font(k: &Kit) -> rune::tok::TextStyle {
    rune::tok::TextStyle { size: 26.0, ..k.ty.body }.lh(34.0)
}

/// A tab body that scrolls inside the window, centred at GitHub's width.
fn scrolling_column(content: El) -> El {
    col()
        .key("detail-scroll")
        .w_full()
        .grow(1.0)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .child(row().w_full().justify_center().padding(24.0).child(col().w_full().max_w(1012.0).child(content)))
}

fn overview(app: &Dialectic, detail: &PullRequestDetail, k: &Kit) -> El {
    let pull_request = &detail.pull_request;
    let s = app.strings();
    if detail.is_editing {
        let edit_actions = row()
            .w_full()
            .gap(8.0)
            .justify_end()
            .child(
                Button::new(s.cancel)
                    .variant(ButtonVariant::Ghost)
                    .on_press(|app: &mut Dialectic| app.cancel_editing_shown_pull_request())
                    .build(k)
                    .key("cancel-edit-button"),
            )
            .child(
                Button::new(s.save)
                    .variant(ButtonVariant::Submit)
                    .on_press(|app: &mut Dialectic| app.save_shown_pull_request_edit())
                    .build(k)
                    .key("save-edit-button"),
            );
        return col()
            .key("edit-form")
            .w_full()
            .gap(12.0)
            .child(markdown_editor::view(&app.draft, s, k))
            .child(edit_actions);
    }
    let description = if pull_request.description.trim().is_empty() {
        text(s.no_description).font(k.ty.ui).color(k.c.text_faint)
    } else {
        markdown_editor::rendered_markdown(&pull_request.description, k)
    };
    let description_card = col()
        .key("description-card")
        .w_full()
        .radius(k.radius.md)
        .border(1.0, k.c.border_weak)
        .child(
            row()
                .w_full()
                .px(16.0)
                .py(10.0)
                .fill(k.c.layer_01)
                .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
                .child(text(pull_request.author.clone()).font(k.ty.ui_medium).color(k.c.text_base))
                .child(text((s.opened_this_on)(&pull_request.created_at)).font(k.ty.ui).color(k.c.text_muted)),
        )
        .child(col().w_full().padding(16.0).child(description));
    col().w_full().gap(16.0).child(description_card).child(merge_box(detail, app.merge_method, s, k))
}

/// GitHub's merge box: the merge button while open, the outcome after.
fn merge_box(detail: &PullRequestDetail, chosen_merge_method: MergeMethod, s: &Strings, k: &Kit) -> El {
    let pull_request = &detail.pull_request;
    let card = |accent: &str| {
        col()
            .key("merge-box")
            .w_full()
            .padding(16.0)
            .gap(12.0)
            .radius(k.radius.md)
            .border(1.0, k.c.border_weak)
            .border_sides([0.0, 0.0, 0.0, 4.0], rune::color::hex(accent).unwrap_or(k.c.border_strong))
    };
    match pull_request.status {
        PullRequestStatus::Merged => card("#8250df").child(heading(
            &(s.merged_into_on)(
                pull_request.merge_method,
                &pull_request.target_branch,
                merged_or_closed_at(pull_request, s),
            ),
            k,
        )),
        PullRequestStatus::Closed => {
            card("#cf222e").child(heading(&(s.closed_without_merging_on)(merged_or_closed_at(pull_request, s)), k))
        }
        PullRequestStatus::Open if !detail.merge_conflicts.is_empty() => {
            let (source_branch, target_branch) = (&pull_request.source_branch, &pull_request.target_branch);
            let mut conflicted_files = col().key("conflicted-files").gap(6.0);
            for conflicted_file in &detail.merge_conflicts {
                conflicted_files = conflicted_files.child(
                    row()
                        .gap(8.0)
                        .items_center()
                        .child(warning_mark(k))
                        .child(text(conflicted_file.clone()).font(k.ty.mono).color(k.c.danger.fg)),
                );
            }
            let agent_prompt = (s.agent_prompt)(source_branch, target_branch, &detail.merge_conflicts.join(", "));
            card("#cf222e")
                .child(row().gap(8.0).items_center().child(warning_mark(k)).child(heading(s.merging_blocked, k)))
                .child(
                    text((s.conflicts_explanation)(source_branch, target_branch))
                        .font(k.ty.ui)
                        .color(k.c.text_muted)
                        .wrap(),
                )
                .child(conflicted_files)
                .child(
                    col()
                        .key("conflict-next-steps")
                        .w_full()
                        .padding(12.0)
                        .gap(8.0)
                        .radius(k.radius.md)
                        .fill(k.c.warning.bg)
                        .border(1.0, k.c.warning.border)
                        .child(text(s.what_to_do).font(k.ty.ui_medium).color(k.c.warning.fg))
                        .child(
                            text((s.contact_branch_owner)(&pull_request.author, source_branch))
                                .font(k.ty.ui)
                                .color(k.c.text_base)
                                .wrap(),
                        )
                        .child(text(s.agent_prompt_intro).font(k.ty.ui).color(k.c.text_base).wrap())
                        .child(
                            col()
                                .key("agent-prompt")
                                .w_full()
                                .padding(8.0)
                                .radius(k.radius.sm)
                                .fill(k.c.layer_01)
                                .border(1.0, k.c.border_weak)
                                .child(text(agent_prompt).font(k.ty.mono).color(k.c.text_base).wrap().selectable(true)),
                        ),
                )
                .child(row().child(close_button(s, k)))
        }
        PullRequestStatus::Open => card("#1f883d")
            .child(heading(s.ready_to_merge, k))
            .child(
                text((s.merge_explanation)(
                    chosen_merge_method,
                    &pull_request.source_branch,
                    &pull_request.target_branch,
                ))
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .wrap(),
            )
            .child(merge_buttons(chosen_merge_method, s, k)),
    }
}

/// The merge method picker over GitHub's merge button (labelled for the chosen method) and Close.
fn merge_buttons(chosen_merge_method: MergeMethod, s: &Strings, k: &Kit) -> El {
    let (merge_label, chosen_index) = match chosen_merge_method {
        MergeMethod::MergeCommit => (s.merge_pull_request, 0),
        MergeMethod::Rebase => (s.rebase_and_merge, 1),
    };
    let method_picker = SegmentedControl::new(&[s.create_a_merge_commit, s.rebase_and_merge], Some(chosen_index))
        .label(s.merge_method)
        .fit()
        .on_pick(|app: &mut Dialectic, picked_method, _: &mut Tx| {
            app.merge_method = if picked_method == 1 { MergeMethod::Rebase } else { MergeMethod::MergeCommit };
        })
        .build(k)
        .key("merge-method");
    let buttons = row()
        .gap(8.0)
        .items_center()
        .child(
            Button::new(merge_label)
                .variant(ButtonVariant::Submit)
                .on_press(|app: &mut Dialectic| app.merge_shown_pull_request())
                .build(k)
                .key("merge-button"),
        )
        .child(close_button(s, k));
    col()
        .gap(12.0)
        .child(
            row()
                .gap(12.0)
                .items_center()
                .child(text(s.merge_method).font(k.ty.ui_medium).color(k.c.text_base))
                .child(method_picker),
        )
        .child(buttons)
}

fn close_button(s: &Strings, k: &Kit) -> El {
    Button::new(s.close_pull_request)
        .variant(ButtonVariant::Outline)
        .on_press(|app: &mut Dialectic| app.close_shown_pull_request())
        .build(k)
        .key("close-button")
}

fn heading(heading_text: &str, k: &Kit) -> El {
    text(heading_text.to_owned()).font(k.ty.body_medium).color(k.c.text_strong)
}

fn merged_or_closed_at<'a>(pull_request: &'a PullRequest, s: &'a Strings) -> &'a str {
    pull_request.merged_or_closed_at.as_deref().unwrap_or(s.unknown_date)
}

fn commit_list(commits: &[CommitSummary], s: &Strings, k: &Kit) -> El {
    let mut list = col().key("commit-list").w_full().radius(k.radius.md).border(1.0, k.c.border_weak);
    if commits.is_empty() {
        list = list.child(col().w_full().padding(16.0).child(text(s.no_commits).font(k.ty.ui).color(k.c.text_faint)));
    }
    for (commit_index, commit) in commits.iter().enumerate() {
        let separator = if commit_index == 0 { 0.0 } else { 1.0 };
        list = list.child(
            row()
                .key(format!("commit-{commit_index}"))
                .w_full()
                .px(16.0)
                .py(10.0)
                .gap(12.0)
                .items_center()
                .border_sides([separator, 0.0, 0.0, 0.0], k.c.border_weak)
                .child(
                    col()
                        .grow(1.0)
                        .gap(2.0)
                        .child(text(commit.subject.clone()).font(k.ty.ui_medium).color(k.c.text_base))
                        .child(
                            text((s.committed_on)(&commit.author, &commit.authored_date))
                                .font(k.ty.caption())
                                .color(k.c.text_muted),
                        ),
                )
                .child(text(commit.short_sha.clone()).font(k.ty.mono).color(k.c.text_muted)),
        );
    }
    list
}

/// GitHub's Files changed: a file list beside the diff. Shared by pull requests and uncommitted changes, so
/// it takes accessors to whichever `DiffState` it shows; `conflicted_files` are flagged in red (and come
/// first, as `diff_state` sorted them).
pub fn files_changed(
    app: &Dialectic,
    get_diff: fn(&Dialectic) -> &DiffState,
    get_diff_mut: fn(&mut Dialectic) -> &mut DiffState,
    conflicted_files: &[String],
    s: &Strings,
    k: &Kit,
) -> El {
    let diff = get_diff(app);
    let (total_additions, total_deletions) = diff
        .files()
        .iter()
        .fold((0, 0), |(additions, deletions), file| (additions + file.additions(), deletions + file.deletions()));
    let is_split = diff.mode() == DiffMode::Split;
    let toolbar = row()
        .key("diff-toolbar")
        .w_full()
        .px(24.0)
        .py(8.0)
        .gap(12.0)
        .items_center()
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(text((s.files_changed_count)(diff.files().len())).font(k.ty.ui_medium).color(k.c.text_base).grow(1.0))
        .child(text(format!("+{total_additions}")).font(k.ty.ui_medium).color(k.c.success.fg))
        .child(text(format!("−{total_deletions}")).font(k.ty.ui_medium).color(k.c.danger.fg))
        .child(
            SegmentedControl::new(&[s.unified, s.split], Some(usize::from(is_split)))
                .label(s.diff_layout)
                .width(Some(160.0))
                .on_pick(|app: &mut Dialectic, picked_layout, tx: &mut Tx| {
                    tx.motion(rune::tok::motion::SNAPPY);
                    app.set_diff_mode(if picked_layout == 1 { DiffMode::Split } else { DiffMode::Unified });
                })
                .build(k),
        );

    let mut file_list = col().key("changed-files").w_full().gap(2.0).padding(8.0);
    for (file_index, file) in diff.files().iter().enumerate() {
        let is_conflicted = conflicted_files.iter().any(|conflicted_file| conflicted_file == file.path());
        let mut file_row = row()
            .key(format!("changed-file-{file_index}"))
            .role("button")
            .label((s.show_file)(file.path(), is_conflicted))
            .w_full()
            .px(8.0)
            .py(6.0)
            .gap(8.0)
            .items_center()
            .radius(k.radius.sm)
            .hover(k.c.hover)
            .on_press_tx(move |app: &mut Dialectic, tx: &mut Tx| get_diff_mut(app).scroll_to_file(file_index, tx));
        if is_conflicted {
            file_row = file_row.fill(k.c.danger.bg).border(1.0, k.c.danger.border).child(warning_mark(k));
        }
        let path_colour = if is_conflicted { k.c.danger.fg } else { k.c.text_base };
        file_list = file_list.child(
            file_row
                .child(text(file.path().to_owned()).font(k.ty.ui).color(path_colour).grow(1.0).wrap_anywhere())
                .child(text(format!("+{}", file.additions())).font(k.ty.caption()).color(k.c.success.fg))
                .child(text(format!("−{}", file.deletions())).font(k.ty.caption()).color(k.c.danger.fg)),
        );
    }

    let diff_view = if diff.files().is_empty() {
        col().grow(1.0).padding(24.0).child(text(s.no_changes).font(k.ty.ui).color(k.c.text_faint))
    } else {
        col().grow(1.0).h_full().child(
            DiffView::new(get_diff, get_diff_mut)
                .key("diff-view")
                .header_actions({
                    let (conflicted_files, conflict_label, k) = (conflicted_files.to_vec(), s.conflict, k.clone());
                    move |app: &Dialectic, file_index| {
                        let is_conflicted = get_diff(app).files().get(file_index).is_some_and(|file| {
                            conflicted_files.iter().any(|conflicted_file| conflicted_file == file.path())
                        });
                        if is_conflicted { conflict_chip(conflict_label, &k) } else { col() }
                    }
                })
                .build(app, k),
        )
    };

    col().w_full().grow(1.0).child(toolbar).child(
        row()
            .w_full()
            .grow(1.0)
            .child(
                col()
                    .w(280.0)
                    .h_full()
                    .fill(k.c.layer_01)
                    .border_sides([0.0, 1.0, 0.0, 0.0], k.c.border_weak)
                    .scroll(rune::ScrollSpec { stick_bottom: false })
                    .child(file_list),
            )
            .child(diff_view),
    )
}

/// The red "Conflict" tag on a conflicting file's diff header.
fn conflict_chip(conflict_label: &'static str, k: &Kit) -> El {
    row()
        .key("conflict-chip")
        .px(8.0)
        .py(2.0)
        .gap(6.0)
        .items_center()
        .radius(999.0)
        .fill(k.c.danger.bg)
        .border(1.0, k.c.danger.border)
        .child(text(conflict_label).font(k.ty.caption()).color(k.c.danger.fg))
}

/// A diff parsed from `git diff` text, conflicting files first, laid out as the user last chose.
pub fn diff_state(diff_text: &str, diff_mode: DiffMode, conflicted_files: &[String]) -> DiffState {
    let mut files = rune_kit::diff::parse(diff_text);
    files.sort_by_key(|file| !conflicted_files.iter().any(|conflicted_file| conflicted_file == file.path()));
    let mut diff = DiffState::new(files);
    diff.set_mode(diff_mode);
    diff
}
