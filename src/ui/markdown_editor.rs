//! GitHub's comment box: a title field and a description with Write and Preview tabs.
//! The new pull request screen and the edit form share one `MarkdownDraft` in the app state.

use super::{Dialectic, Strings, tab_strip};
use rune::{El, TextEdit, col, text};
use rune_kit::markdown::Markdown;
use rune_kit::{Kit, TextArea, TextInput};

#[derive(Default)]
pub struct MarkdownDraft {
    pub title: TextEdit,
    pub description: TextEdit,
    pub showing_preview: bool,
}

impl MarkdownDraft {
    pub fn with_text(title: &str, description: &str) -> MarkdownDraft {
        MarkdownDraft { title: TextEdit::new(title), description: TextEdit::new(description), showing_preview: false }
    }
}

pub fn view(draft: &MarkdownDraft, s: &Strings, k: &Kit) -> El {
    let title_input = TextInput::new(&draft.title)
        .label(s.title)
        .placeholder(s.title)
        .large()
        .width(None)
        .on_edit(|app: &mut Dialectic| &mut app.draft.title)
        .build(k);

    let tabs = tab_strip(
        k,
        "description-tab",
        &[s.write, s.preview],
        usize::from(draft.showing_preview),
        |app: &mut Dialectic, picked_tab| app.draft.showing_preview = picked_tab == 1,
    );
    let tab_body = if draft.showing_preview {
        let preview = if draft.description.text.trim().is_empty() {
            text(s.nothing_to_preview).font(k.ty.ui).color(k.c.text_faint)
        } else {
            rendered_markdown(&draft.description.text, k)
        };
        col().key("description-preview").w_full().min_h(240.0).padding(12.0).child(preview)
    } else {
        col().w_full().padding(8.0).child(
            TextArea::new(&draft.description)
                .label(s.description)
                .placeholder(s.description_placeholder)
                .width(None)
                .rows(12)
                .on_edit(|app: &mut Dialectic| &mut app.draft.description)
                .build(k),
        )
    };

    col().w_full().gap(12.0).child(title_input).child(
        col()
            .w_full()
            .radius(k.radius.md)
            .border(1.0, k.c.border_weak)
            .child(col().w_full().px(8.0).fill(k.c.layer_01).child(tabs))
            .child(tab_body),
    )
}

/// A pull request description as GitHub renders it; task checkboxes show their state but don't toggle.
pub fn rendered_markdown(source: &str, k: &Kit) -> El {
    Markdown::new(source).tasks_read_only().selectable(true).build(k)
}
