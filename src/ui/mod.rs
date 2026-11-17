//! The desktop app: one window whose `Screen` picks what `view` draws.

mod markdown_editor;
mod new_pull_request;
mod pull_request_list;
mod welcome;

use crate::db::{ProjectStore, PullRequest, PullRequestStatus, SavedProject};
use crate::repository::Repository;
use markdown_editor::MarkdownDraft;
use new_pull_request::NewPullRequestForm;
use rune::{App, AppSpec, Cx, El, col, row, text};
use rune_kit::Kit;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    PullRequestList,
    NewPullRequest,
}

pub struct Dialectic {
    pub screen: Screen,
    /// `None` when the app data directory couldn't be opened; `error_message` says why.
    pub project_store: Option<ProjectStore>,
    pub saved_projects: Vec<SavedProject>,
    pub repository: Option<Repository>,
    /// Reloaded each time the list shows, so pull requests opened elsewhere (another app, the CLI) appear.
    pub pull_requests: Vec<PullRequest>,
    pub new_pull_request_form: NewPullRequestForm,
    /// The title and description being written: a new pull request's, or an open one's while editing.
    pub draft: MarkdownDraft,
    /// The last failure, shown on the current screen until the next action.
    pub error_message: Option<String>,
}

impl Dialectic {
    /// Opens the repository named by the first argument Rune doesn't use itself, else the welcome screen.
    fn from_command_line() -> Dialectic {
        let mut app = Dialectic::without_repository();
        if let Some(repository_path) = rune::extra_args().iter().find(|argument| !argument.starts_with('-')) {
            app.open_repository(Path::new(repository_path));
        }
        app
    }

    pub fn without_repository() -> Dialectic {
        let mut app = Dialectic {
            screen: Screen::Welcome,
            project_store: None,
            saved_projects: Vec::new(),
            repository: None,
            pull_requests: Vec::new(),
            new_pull_request_form: NewPullRequestForm::default(),
            draft: MarkdownDraft::default(),
            error_message: None,
        };
        match ProjectStore::open_in_app_data_dir() {
            Ok(project_store) => app.project_store = Some(project_store),
            Err(error) => app.error_message = Some(format!("Saved projects are unavailable: {error}")),
        }
        app.reload_saved_projects();
        app
    }

    pub fn reload_saved_projects(&mut self) {
        if let Some(project_store) = &self.project_store {
            match project_store.list_most_recent_first() {
                Ok(saved_projects) => self.saved_projects = saved_projects,
                Err(error) => self.error_message = Some(error.to_string()),
            }
        }
    }

    pub fn open_repository(&mut self, any_path_inside: &Path) {
        match Repository::open(any_path_inside) {
            Ok(repository) => {
                if let Some(project_store) = &self.project_store
                    && let Err(error) = project_store.record_opened(&repository.top_level_path)
                {
                    self.error_message = Some(error.to_string());
                }
                self.repository = Some(repository);
                self.show_pull_request_list();
            }
            Err(error) => {
                self.error_message = Some(error.to_string());
                self.screen = Screen::Welcome;
            }
        }
    }

    pub fn show_welcome(&mut self) {
        self.repository = None;
        self.error_message = None;
        self.reload_saved_projects();
        self.screen = Screen::Welcome;
    }

    /// Starts from the current branch into `main` (or `master`) when those exist.
    pub fn show_new_pull_request(&mut self) {
        let Some(repository) = &self.repository else { return };
        let local_branches = match repository.local_branches() {
            Ok(local_branches) => local_branches,
            Err(error) => {
                self.error_message = Some(error.to_string());
                return;
            }
        };
        let target_branch = ["main", "master"]
            .into_iter()
            .find(|default_branch| local_branches.iter().any(|branch| branch == default_branch));
        let source_branch = repository
            .current_branch()
            .filter(|current_branch| Some(current_branch.as_str()) != target_branch);
        self.new_pull_request_form = NewPullRequestForm {
            source_branch,
            target_branch: target_branch.map(str::to_owned),
            local_branches,
            open_branch_picker: None,
        };
        self.draft = MarkdownDraft::default();
        self.error_message = None;
        self.screen = Screen::NewPullRequest;
    }

    pub fn create_pull_request_from_draft(&mut self) {
        let Some(repository) = &self.repository else { return };
        let form = &self.new_pull_request_form;
        let (Some(source_branch), Some(target_branch)) = (&form.source_branch, &form.target_branch) else {
            self.error_message = Some("Choose both a source and a target branch.".into());
            return;
        };
        match repository.create_pull_request(
            &self.draft.title.text,
            &self.draft.description.text,
            source_branch,
            target_branch,
        ) {
            Ok(_) => self.show_pull_request_list(),
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    pub fn show_pull_request_list(&mut self) {
        self.error_message = None;
        self.screen = Screen::PullRequestList;
        if let Some(repository) = &self.repository {
            match repository.list_pull_requests() {
                Ok(pull_requests) => self.pull_requests = pull_requests,
                Err(error) => self.error_message = Some(error.to_string()),
            }
        }
    }
}

pub fn kit(cx: &Cx<Dialectic>) -> Kit {
    Kit::neutral(cx.system_dark())
}

fn view(app: &Dialectic, cx: &mut Cx<Dialectic>) -> El {
    let k = kit(cx);
    let mut overlay_layer = Vec::new();
    let screen = match app.screen {
        Screen::Welcome => welcome::view(app, &k),
        Screen::PullRequestList => pull_request_list::view(app, &k),
        Screen::NewPullRequest => new_pull_request::view(app, &k, &mut overlay_layer),
    };
    rune_kit::root(col().w_full().h_full().fill(k.c.bg_base).child(screen), overlay_layer)
}

/// `repository-name / Pull requests`, GitHub's breadcrumb: the name returns to the welcome screen.
fn repository_header(app: &Dialectic, k: &Kit) -> El {
    let repository_name = app.repository.as_ref().map(Repository::name).unwrap_or_default();
    row()
        .key("repository-header")
        .w_full()
        .h(52.0)
        .px(24.0)
        .gap(8.0)
        .items_center()
        .fill(k.c.layer_01)
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(
            text(repository_name)
                .key("repository-name")
                .font(k.ty.body_medium)
                .color(k.c.text_accent)
                .on_press(|app: &mut Dialectic| app.show_welcome()),
        )
        .child(text("/").font(k.ty.body).color(k.c.text_faint))
        .child(
            text("Pull requests")
                .key("pull-requests-link")
                .font(k.ty.body_medium)
                .color(k.c.text_base)
                .on_press(|app: &mut Dialectic| app.show_pull_request_list()),
        )
}

/// A repository screen: the header over a scrolling, centred column at most 1012 points wide, as GitHub's.
pub fn page(app: &Dialectic, k: &Kit, content: El) -> El {
    col().w_full().h_full().child(repository_header(app, k)).child(
        col()
            .key("page-scroll")
            .w_full()
            .grow(1.0)
            .scroll(rune::ScrollSpec { stick_bottom: false })
            .child(
                row()
                    .w_full()
                    .justify_center()
                    .padding(24.0)
                    .child(col().w_full().max_w(1012.0).child(content)),
            ),
    )
}

/// GitHub's state colours: green open, purple merged, red closed.
pub fn status_badge(status: PullRequestStatus, k: &Kit) -> El {
    let (label, colour) = match status {
        PullRequestStatus::Open => ("Open", "#1f883d"),
        PullRequestStatus::Merged => ("Merged", "#8250df"),
        PullRequestStatus::Closed => ("Closed", "#cf222e"),
    };
    row()
        .key("status-badge")
        .px(12.0)
        .py(4.0)
        .radius(999.0)
        .fill(rune::color::hex(colour).unwrap_or(k.c.text_muted))
        .child(text(label).font(k.ty.ui_medium).color(k.c.white))
}

/// `source → target` as two monospace branch chips.
pub fn branch_flow(source_branch: &str, target_branch: &str, k: &Kit) -> El {
    let chip = |branch_name: &str| {
        row()
            .px(6.0)
            .py(2.0)
            .radius(k.radius.sm)
            .fill(k.c.info.bg)
            .child(text(branch_name.to_owned()).font(k.ty.mono).color(k.c.info.fg))
    };
    row()
        .gap(6.0)
        .items_center()
        .child(chip(source_branch))
        .child(text("→").font(k.ty.ui).color(k.c.text_muted))
        .child(chip(target_branch))
}

/// GitHub's underlined tabs. `on_pick` gets the picked tab's index.
pub fn tab_strip(
    k: &Kit,
    key_prefix: &str,
    tab_labels: &[&str],
    selected_tab: usize,
    on_pick: fn(&mut Dialectic, usize),
) -> El {
    let mut strip = row().key(key_prefix.to_owned()).gap(4.0);
    for (tab_index, tab_label) in tab_labels.iter().enumerate() {
        let is_selected = tab_index == selected_tab;
        let underline = if is_selected {
            rune::color::hex("#fd8c73").unwrap_or(k.c.text_accent)
        } else {
            k.c.bg_base.alpha(0.0)
        };
        strip = strip.child(
            col()
                .key(format!("{key_prefix}-{tab_index}"))
                .role("tab")
                .label((*tab_label).to_owned())
                .checked(is_selected)
                .focusable(true)
                .px(12.0)
                .pt(10.0)
                .gap(8.0)
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .on_press(move |app: &mut Dialectic| on_pick(app, tab_index))
                .child(
                    text((*tab_label).to_owned())
                        .font(if is_selected { k.ty.ui_medium } else { k.ty.ui })
                        .color(if is_selected { k.c.text_strong } else { k.c.text_muted }),
                )
                .child(col().w_full().h(2.0).radius(1.0).fill(underline)),
        );
    }
    strip
}

/// A one-line error in the danger colour, or nothing.
pub fn error_banner(app: &Dialectic, k: &Kit) -> Option<El> {
    app.error_message.as_ref().map(|message| {
        row()
            .key("error-banner")
            .w_full()
            .padding(12.0)
            .radius(k.radius.md)
            .fill(k.c.danger.bg)
            .border(1.0, k.c.danger.border)
            .child(text(message.clone()).font(k.ty.ui).color(k.c.danger.fg).wrap())
    })
}

/// A description using every Markdown feature the editor promises, for scenes and the demo.
pub const SAMPLE_DESCRIPTION: &str = "## What changed

Greetings now end with punctuation, `!` by default.

- [x] Add a `punctuation` parameter
- [ ] Document it in the [README](https://example.com/readme)

1. Call `greet('Ada')`
2. See `Hello, Ada!`

```python
greet('Ada', punctuation='?')  # Hello, Ada?
```
";

/// Headless stills of each screen: `dialectic <repo> --snap out.png --scene <name>` (see `make screenshots`).
const SCENES: &[(&str, fn(&mut Dialectic))] = &[
    ("new-pull-request", |app| {
        app.show_new_pull_request();
        app.draft = MarkdownDraft::with_text("Add punctuation to greetings", SAMPLE_DESCRIPTION);
    }),
    ("new-pull-request-preview", |app| {
        app.show_new_pull_request();
        app.draft = MarkdownDraft::with_text("Add punctuation to greetings", SAMPLE_DESCRIPTION);
        app.draft.showing_preview = true;
    }),
];

pub fn run() {
    let spec = AppSpec {
        scenes: SCENES,
        ..AppSpec::new("Dialectic", (1100.0, 760.0), Dialectic::from_command_line, view)
    };
    App::new(spec).modifier_keys().run();
}
