//! "Open a pull request": two local branches, a title and a Markdown description.

use super::{Dialectic, error_banner, markdown_editor, page};
use rune::{El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Select};

#[derive(Default)]
pub struct NewPullRequestForm {
    pub local_branches: Vec<String>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    /// Which branch picker is open: "source-branch" or "target-branch".
    pub open_branch_picker: Option<&'static str>,
}

pub fn view(app: &Dialectic, k: &Kit, overlay_layer: &mut Vec<El>) -> El {
    let form = &app.new_pull_request_form;
    let branch_picker = |picker_key: &'static str, current_branch: &Option<String>, overlay_layer: &mut Vec<El>| {
        Select::new(picker_key, &form.local_branches)
            .label(if picker_key == "source-branch" { "Source branch" } else { "Target branch" })
            .placeholder("Choose a branch")
            .current(current_branch.as_deref())
            .open(form.open_branch_picker == Some(picker_key))
            .on_toggle(move |app: &mut Dialectic| {
                let picker = &mut app.new_pull_request_form.open_branch_picker;
                *picker = if *picker == Some(picker_key) { None } else { Some(picker_key) };
            })
            .on_close(|app: &mut Dialectic| app.new_pull_request_form.open_branch_picker = None)
            .on_choose(move |app: &mut Dialectic, chosen_branch: &str, _: &mut Tx| {
                let form = &mut app.new_pull_request_form;
                let chosen_branch = Some(chosen_branch.to_owned());
                if picker_key == "source-branch" {
                    form.source_branch = chosen_branch
                } else {
                    form.target_branch = chosen_branch
                }
                form.open_branch_picker = None;
            })
            .build(k, overlay_layer)
    };

    let branch_row = row()
        .key("branch-row")
        .w_full()
        .padding(12.0)
        .gap(8.0)
        .items_center()
        .radius(k.radius.md)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_weak)
        .child(text("Merge").font(k.ty.ui).color(k.c.text_muted))
        .child(branch_picker("source-branch", &form.source_branch, overlay_layer))
        .child(text("→ into").font(k.ty.ui).color(k.c.text_muted))
        .child(branch_picker("target-branch", &form.target_branch, overlay_layer));

    let actions = row()
        .w_full()
        .gap(8.0)
        .justify_end()
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Ghost)
                .on_press(|app: &mut Dialectic| app.show_pull_request_list())
                .build(k),
        )
        .child(
            Button::new("Create pull request")
                .variant(ButtonVariant::Submit)
                .on_press(|app: &mut Dialectic| app.create_pull_request_from_draft())
                .build(k)
                .key("create-button"),
        );

    let mut content = col()
        .w_full()
        .gap(16.0)
        .child(text("Open a pull request").font(k.ty.body_medium).color(k.c.text_strong))
        .child(
            text("Pick the branch with the changes and the branch they should merge into.")
                .font(k.ty.ui)
                .color(k.c.text_muted),
        )
        .child(branch_row);
    if let Some(banner) = error_banner(app, k) {
        content = content.child(banner);
    }
    page(app, k, content.child(markdown_editor::view(&app.draft, k)).child(actions))
}
