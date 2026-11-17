//! One pull request: GitHub's header and its Overview, Commits and Files changed tabs.

use super::{Dialectic, branch_flow, error_banner, markdown_editor, repository_header, status_badge, tab_strip};
use crate::db::{PullRequest, PullRequestStatus};
use crate::git::CommitSummary;
use rune::{El, Tx, col, row, text};
use rune_kit::diff::{DiffMode, DiffState, DiffView};
use rune_kit::{Kit, SegmentedControl};

pub const OVERVIEW_TAB: usize = 0;
pub const COMMITS_TAB: usize = 1;
pub const FILES_CHANGED_TAB: usize = 2;

pub struct PullRequestDetail {
    pub pull_request: PullRequest,
    pub commits: Vec<CommitSummary>,
    pub selected_tab: usize,
}

pub fn view(app: &Dialectic, detail: &PullRequestDetail, k: &Kit) -> El {
    let pull_request = &detail.pull_request;
    let changed_file_count = app.pull_request_diff.files().len();
    let tabs = tab_strip(
        k,
        "detail-tab",
        &[
            "Overview".to_owned(),
            format!("Commits {}", detail.commits.len()),
            format!("Files changed {changed_file_count}"),
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
                .child(text(format!("#{}", pull_request.id)).font(title_font(k)).color(k.c.text_faint)),
        )
        .child(
            row()
                .gap(8.0)
                .items_center()
                .child(status_badge(pull_request.status, k))
                .child(text(activity_sentence(pull_request, detail.commits.len())).font(k.ty.ui).color(k.c.text_muted))
                .child(branch_flow(&pull_request.source_branch, &pull_request.target_branch, k)),
        );
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
        FILES_CHANGED_TAB => files_changed(app, k),
        COMMITS_TAB => scrolling_column(commit_list(&detail.commits, k)),
        _ => scrolling_column(overview(detail, k)),
    };
    col().w_full().h_full().child(repository_header(app, k)).child(top).child(tab_body)
}

fn title_font(k: &Kit) -> rune::tok::TextStyle {
    rune::tok::TextStyle { size: 26.0, ..k.ty.body }.lh(34.0)
}

/// "Linus wants to merge 2 commits:", followed by the `source → target` chips.
fn activity_sentence(pull_request: &PullRequest, commit_count: usize) -> String {
    let commits = if commit_count == 1 { "1 commit".to_owned() } else { format!("{commit_count} commits") };
    let verb = match pull_request.status {
        PullRequestStatus::Open => "wants to merge",
        PullRequestStatus::Merged => "merged",
        PullRequestStatus::Closed => "proposed merging",
    };
    format!("{} {verb} {commits}:", pull_request.author)
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

fn overview(detail: &PullRequestDetail, k: &Kit) -> El {
    let pull_request = &detail.pull_request;
    let description = if pull_request.description.trim().is_empty() {
        text("No description provided.").font(k.ty.ui).color(k.c.text_faint)
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
                .child(
                    text(format!(" opened this on {} UTC", pull_request.created_at))
                        .font(k.ty.ui)
                        .color(k.c.text_muted),
                ),
        )
        .child(col().w_full().padding(16.0).child(description));
    col().w_full().gap(16.0).child(description_card)
}

fn commit_list(commits: &[CommitSummary], k: &Kit) -> El {
    let mut list = col().key("commit-list").w_full().radius(k.radius.md).border(1.0, k.c.border_weak);
    if commits.is_empty() {
        list = list.child(col().w_full().padding(16.0).child(text("No commits.").font(k.ty.ui).color(k.c.text_faint)));
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
                            text(format!("{} committed on {}", commit.author, commit.authored_date))
                                .font(k.ty.caption())
                                .color(k.c.text_muted),
                        ),
                )
                .child(text(commit.short_sha.clone()).font(k.ty.mono).color(k.c.text_muted)),
        );
    }
    list
}

fn files_changed(app: &Dialectic, k: &Kit) -> El {
    let diff = &app.pull_request_diff;
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
        .child(
            text(format!("{} files changed", diff.files().len())).font(k.ty.ui_medium).color(k.c.text_base).grow(1.0),
        )
        .child(text(format!("+{total_additions}")).font(k.ty.ui_medium).color(k.c.success.fg))
        .child(text(format!("−{total_deletions}")).font(k.ty.ui_medium).color(k.c.danger.fg))
        .child(
            SegmentedControl::new(&["Unified", "Split"], Some(usize::from(is_split)))
                .label("Diff layout")
                .width(Some(160.0))
                .on_pick(|app: &mut Dialectic, picked_layout, tx: &mut Tx| {
                    tx.motion(rune::tok::motion::SNAPPY);
                    app.set_diff_mode(if picked_layout == 1 { DiffMode::Split } else { DiffMode::Unified });
                })
                .build(k),
        );

    let mut file_list = col().key("changed-files").w_full().gap(2.0).padding(8.0);
    for (file_index, file) in diff.files().iter().enumerate() {
        file_list = file_list.child(
            row()
                .key(format!("changed-file-{file_index}"))
                .role("button")
                .label(format!("Show {}", file.path()))
                .w_full()
                .px(8.0)
                .py(6.0)
                .gap(8.0)
                .items_center()
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .on_press_tx(move |app: &mut Dialectic, tx: &mut Tx| {
                    app.pull_request_diff.scroll_to_file(file_index, tx)
                })
                .child(text(file.path().to_owned()).font(k.ty.ui).color(k.c.text_base).grow(1.0).wrap_anywhere())
                .child(text(format!("+{}", file.additions())).font(k.ty.caption()).color(k.c.success.fg))
                .child(text(format!("−{}", file.deletions())).font(k.ty.caption()).color(k.c.danger.fg)),
        );
    }

    let diff_view = if diff.files().is_empty() {
        col().grow(1.0).padding(24.0).child(text("No changes.").font(k.ty.ui).color(k.c.text_faint))
    } else {
        col().grow(1.0).h_full().child(
            DiffView::new(|app: &Dialectic| &app.pull_request_diff, |app: &mut Dialectic| &mut app.pull_request_diff)
                .key("diff-view")
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

/// A diff parsed from `git diff` text, laid out as the user last chose.
pub fn diff_state(diff_text: &str, diff_mode: DiffMode) -> DiffState {
    let mut diff = DiffState::new(rune_kit::diff::parse(diff_text));
    diff.set_mode(diff_mode);
    diff
}
