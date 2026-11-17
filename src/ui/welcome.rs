//! Xcode-style welcome: the app and "Open Repository…" on the left, saved projects on the right.

use super::{Dialectic, error_banner};
use crate::db::SavedProject;
use rune::{El, FileDialogOptions, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(app: &Dialectic, k: &Kit) -> El {
    let open_repository_button = Button::new("Open Repository…")
        .variant(ButtonVariant::Contrast)
        .on_press_tx(|_: &mut Dialectic, tx: &mut Tx| {
            let options = FileDialogOptions {
                title: "Open a Git repository".into(),
                ..Default::default()
            };
            tx.open_folder(
                options,
                |app: &mut Dialectic, picked_folder, _: &mut Tx| match picked_folder {
                    Ok(Some(folder)) => app.open_repository(&folder),
                    Ok(None) => {}
                    Err(error) => app.error_message = Some(format!("Could not show the folder picker: {error}")),
                },
            );
        })
        .build(k);

    let mut introduction = col()
        .key("welcome-introduction")
        .w(360.0)
        .h_full()
        .padding(32.0)
        .gap(12.0)
        .center()
        .child(
            text("Dialectic")
                .font(
                    rune::tok::TextStyle {
                        size: 32.0,
                        ..k.ty.body_medium
                    }
                    .lh(40.0),
                )
                .color(k.c.text_strong),
        )
        .child(
            text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                .font(k.ty.ui)
                .color(k.c.text_muted),
        )
        .child(
            text("Local pull requests between the branches of one repository.")
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(col().h(12.0))
        .child(open_repository_button);
    if let Some(banner) = error_banner(app, k) {
        introduction = introduction.child(banner);
    }

    let mut project_rows = col().key("saved-projects").w_full().gap(2.0);
    if app.saved_projects.is_empty() {
        project_rows = project_rows.child(
            text("No repositories yet. Open one to start.")
                .font(k.ty.ui)
                .color(k.c.text_faint)
                .wrap(),
        );
    }
    for (index, saved_project) in app.saved_projects.iter().enumerate() {
        project_rows = project_rows.child(saved_project_row(index, saved_project, k));
    }

    row().w_full().h_full().child(introduction).child(
        col()
            .grow(1.0)
            .h_full()
            .padding(16.0)
            .gap(8.0)
            .fill(k.c.layer_01)
            .border_sides([0.0, 0.0, 0.0, 1.0], k.c.border_weak)
            .child(
                text("Recent repositories")
                    .font(k.ty.small_medium)
                    .color(k.c.text_faint),
            )
            .child(
                col()
                    .grow(1.0)
                    .w_full()
                    .scroll(rune::ScrollSpec { stick_bottom: false })
                    .child(project_rows),
            ),
    )
}

fn saved_project_row(index: usize, saved_project: &SavedProject, k: &Kit) -> El {
    let repository_path = saved_project.repository_path.clone();
    let is_missing = !repository_path.exists();
    let folder_name = repository_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| repository_path.display().to_string());
    let name_color = if is_missing { k.c.text_faint } else { k.c.text_base };

    let mut details = col()
        .grow(1.0)
        .gap(2.0)
        .child(text(folder_name).font(k.ty.ui_medium).color(name_color))
        .child(
            text(repository_path.display().to_string())
                .font(k.ty.caption())
                .color(k.c.text_faint),
        );
    if is_missing {
        details = details.child(text("Missing").font(k.ty.caption()).color(k.c.danger.fg));
    }

    let removed_path = repository_path.clone();
    let remove_button = Button::new("Remove")
        .variant(ButtonVariant::GhostFaint)
        .on_press(move |app: &mut Dialectic| {
            if let Some(project_store) = &app.project_store
                && let Err(error) = project_store.remove(&removed_path)
            {
                app.error_message = Some(error.to_string());
            }
            app.reload_saved_projects();
        })
        .build(k);

    let mut project_row = row()
        .key(format!("saved-project-{index}"))
        .role("button")
        .label(format!("Open {}", repository_path.display()))
        .w_full()
        .px(12.0)
        .py(8.0)
        .gap(12.0)
        .items_center()
        .radius(k.radius.md)
        .hover(k.c.hover)
        .child(details)
        .child(remove_button);
    if !is_missing {
        project_row = project_row.on_press(move |app: &mut Dialectic| app.open_repository(&repository_path));
    }
    project_row
}
