//! GitHub's pull request list: open and finished pull requests with their branches.

use super::{Dialectic, Strings, error_banner, page, status_badge};
use crate::db::{PullRequest, PullRequestStatus};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(app: &Dialectic, k: &Kit) -> El {
    let s = app.strings();
    let open_count =
        app.pull_requests.iter().filter(|pull_request| pull_request.status == PullRequestStatus::Open).count();
    let finished_count = app.pull_requests.len() - open_count;

    let title_row = row()
        .w_full()
        .items_center()
        .child(text(s.pull_requests).font(k.ty.body_medium).color(k.c.text_strong).grow(1.0))
        .child(
            Button::new(s.new_pull_request)
                .variant(ButtonVariant::Submit)
                .on_press(|app: &mut Dialectic| app.show_new_pull_request())
                .build(k)
                .key("new-pull-request-button"),
        );

    let mut table = col()
        .key("pull-request-table")
        .w_full()
        .radius(k.radius.md)
        .border(1.0, k.c.border_weak)
        .clip_rounded(k.radius.md)
        .child(
            row()
                .w_full()
                .px(16.0)
                .py(12.0)
                .gap(16.0)
                .fill(k.c.layer_01)
                .child(text((s.open_count)(open_count)).font(k.ty.ui_medium).color(k.c.text_base))
                .child(text((s.merged_or_closed_count)(finished_count)).font(k.ty.ui).color(k.c.text_muted)),
        );
    if app.pull_requests.is_empty() {
        table = table.child(
            col()
                .w_full()
                .padding(32.0)
                .center()
                .border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_weak)
                .child(text(s.no_pull_requests_yet).font(k.ty.body_medium).color(k.c.text_base))
                .child(text(s.no_pull_requests_hint).font(k.ty.ui).color(k.c.text_muted)),
        );
    }
    for pull_request in &app.pull_requests {
        table = table.child(pull_request_row(pull_request, s, k));
    }

    let mut content = col().w_full().gap(16.0).child(title_row);
    if let Some(banner) = error_banner(app, k) {
        content = content.child(banner);
    }
    page(content.child(table))
}

fn pull_request_row(pull_request: &PullRequest, s: &Strings, k: &Kit) -> El {
    let pull_request_id = pull_request.id;
    let (activity_status, activity_date) = match &pull_request.merged_or_closed_at {
        Some(merged_or_closed_at) if pull_request.status != PullRequestStatus::Open => {
            (pull_request.status, merged_or_closed_at)
        }
        _ => (PullRequestStatus::Open, &pull_request.created_at),
    };
    row()
        .key(format!("pull-request-{pull_request_id}"))
        .role("link")
        .label(format!("Pull request #{pull_request_id}: {}", pull_request.title))
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .items_center()
        .border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_weak)
        .hover(k.c.hover)
        .on_press(move |app: &mut Dialectic| app.show_pull_request(pull_request_id))
        .child(status_badge(pull_request.status, s, k))
        .child(
            col()
                .grow(1.0)
                .gap(4.0)
                .child(text(pull_request.title.clone()).font(k.ty.body_medium).color(k.c.text_strong))
                .child(
                    text((s.pull_request_byline)(
                        pull_request_id,
                        activity_status,
                        activity_date,
                        &pull_request.author,
                    ))
                    .font(k.ty.caption())
                    .color(k.c.text_muted),
                ),
        )
        .child(super::branch_flow(&pull_request.source_branch, &pull_request.target_branch, k))
}
