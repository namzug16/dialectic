//! Settings, reached from the welcome screen: the app's preferences and what it is (version, source).

use super::{Dialectic, Language, Strings, TITLEBAR_HEIGHT, error_banner, icons};
use rune::{El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl};

pub fn view(app: &Dialectic, k: &Kit) -> El {
    let s = app.strings();
    let header = row()
        .w_full()
        .gap(12.0)
        .items_center()
        .child(
            Button::new(s.back_to_repositories)
                .variant(ButtonVariant::Ghost)
                .icon(&icons::ARROW_LEFT)
                .help(s.back_to_repositories_help)
                .on_press(|app: &mut Dialectic| app.show_welcome())
                .build(k)
                .key("back-to-repositories"),
        )
        .child(text(s.settings).font(k.ty.body_medium).color(k.c.text_strong));

    let source_code_url = env!("CARGO_PKG_REPOSITORY");
    let about = section(
        s.about,
        vec![
            setting_row(
                "Dialectic",
                text((s.version)(env!("CARGO_PKG_VERSION"))).font(k.ty.ui).color(k.c.text_muted),
                k,
            ),
            setting_row(
                s.source_code,
                Button::new(source_code_url.trim_start_matches("https://"))
                    .variant(ButtonVariant::Ghost)
                    .icon(&icons::EXTERNAL_LINK)
                    .on_press_tx(move |app: &mut Dialectic, tx: &mut Tx| {
                        if let Err(error) = tx.open_url(source_code_url) {
                            app.error_message = Some((app.strings().open_link_failed)(&error.to_string()));
                        }
                    })
                    .build(k)
                    .key("source-code-link"),
                k,
            ),
        ],
        k,
    );

    let mut content = col()
        .w_full()
        .max_w(640.0)
        .gap(20.0)
        .child(header)
        .child(section(s.language, vec![setting_row(s.language, language_picker(app, s, k), k)], k))
        .child(about);
    if let Some(banner) = error_banner(app, k) {
        content = content.child(banner);
    }
    // No title bar on this screen: the top padding clears the traffic lights, and the whole screen drags.
    col()
        .key("settings")
        .w_full()
        .h_full()
        .window_drag()
        .pt(TITLEBAR_HEIGHT)
        .px(24.0)
        .items_center()
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .child(content)
}

/// A titled card of setting rows.
fn section(title: &str, setting_rows: Vec<El>, k: &Kit) -> El {
    col().w_full().gap(8.0).child(text(title.to_owned()).font(k.ty.small_medium).color(k.c.text_faint)).child(
        col().w_full().radius(k.radius.md).border(1.0, k.c.border_weak).fill(k.c.layer_01).children(setting_rows),
    )
}

/// A setting's name on the left, its control or value on the right.
fn setting_row(name: &str, control: El, k: &Kit) -> El {
    row()
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .items_center()
        .child(text(name.to_owned()).font(k.ty.ui).color(k.c.text_base).grow(1.0))
        .child(control)
}

/// English or Español, each named in itself so anyone can find their own.
fn language_picker(app: &Dialectic, s: &Strings, k: &Kit) -> El {
    let language_names = Language::ALL.map(Language::native_name);
    let selected_index = Language::ALL.iter().position(|language| *language == app.language);
    SegmentedControl::new(&language_names, selected_index)
        .label(s.language)
        .fit()
        .on_pick(|app: &mut Dialectic, picked_index, _: &mut Tx| app.set_language(Language::ALL[picked_index]))
        .build(k)
        .key("language-picker")
}
