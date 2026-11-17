//! The desktop app: one window whose `Screen` picks what `view` draws.

mod markdown_editor;
mod new_pull_request;
mod pull_request_detail;
mod pull_request_list;
mod welcome;

use crate::db::{MergeMethod, ProjectStore, PullRequest, PullRequestStatus, SavedProject};
use crate::repository::{MergeOutcome, Repository};
use markdown_editor::MarkdownDraft;
use new_pull_request::NewPullRequestForm;
use pull_request_detail::PullRequestDetail;
use rune::{App, AppSpec, Cx, El, KeyEvent, Tx, col, row, text};
use rune_kit::diff::{DiffMode, DiffState};
use rune_kit::{Button, ButtonVariant, Kit};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    PullRequestList,
    NewPullRequest,
    PullRequestDetail,
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
    /// The merge button's method; starts as a merge commit each launch.
    pub merge_method: MergeMethod,
    pub pull_request_detail: Option<PullRequestDetail>,
    /// The shown pull request's diff; kept outside `pull_request_detail` because `DiffView` borrows it by accessor.
    pub pull_request_diff: DiffState,
    /// The last failure, shown on the current screen until the next action.
    pub error_message: Option<String>,
}

const DIFF_MODE_SETTING: &str = "diff_mode";

impl Dialectic {
    /// Opens the repository named by the first argument Rune doesn't use itself, else the welcome screen.
    fn from_command_line() -> Dialectic {
        let mut app = Dialectic::without_repository();
        let extra_args = rune::extra_args();
        let repository_path = extra_args
            .iter()
            .enumerate()
            .find(|(index, argument)| {
                !argument.starts_with('-') && (*index == 0 || extra_args[index - 1] != "--pull-request")
            })
            .map(|(_, argument)| argument);
        if let Some(repository_path) = repository_path {
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
            merge_method: MergeMethod::MergeCommit,
            pull_request_detail: None,
            pull_request_diff: DiffState::new(Vec::new()),
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
        let source_branch =
            repository.current_branch().filter(|current_branch| Some(current_branch.as_str()) != target_branch);
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
            Ok(pull_request_id) => self.show_pull_request(pull_request_id),
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    /// Loads the pull request with its commits and diff, opening on the Overview tab.
    pub fn show_pull_request(&mut self, pull_request_id: i64) {
        self.error_message = None;
        if let Err(error) = self.load_pull_request(pull_request_id, pull_request_detail::OVERVIEW_TAB) {
            self.error_message = Some(error.to_string());
            return;
        }
        self.screen = Screen::PullRequestDetail;
    }

    fn load_pull_request(&mut self, pull_request_id: i64, selected_tab: usize) -> crate::Result<()> {
        let Some(repository) = &self.repository else { return Ok(()) };
        let pull_request = repository.pull_request(pull_request_id)?;
        let commits = repository.commits(&pull_request)?;
        let merge_conflicts = repository.merge_conflicts(&pull_request)?;
        self.pull_request_diff =
            pull_request_detail::diff_state(&repository.diff(&pull_request)?, self.saved_diff_mode(), &merge_conflicts);
        self.pull_request_detail =
            Some(PullRequestDetail { pull_request, commits, selected_tab, merge_conflicts, is_editing: false });
        Ok(())
    }

    /// Merges with git; on conflicts the pull request stays open and the files show in the merge box.
    pub fn merge_shown_pull_request(&mut self) {
        let (Some(repository), Some(detail)) = (&self.repository, &self.pull_request_detail) else { return };
        let pull_request_id = detail.pull_request.id;
        self.error_message = None;
        match repository.merge_pull_request(pull_request_id, self.merge_method) {
            Ok(MergeOutcome::Merged) => self.reload_shown_pull_request(),
            // Conflicts the prediction missed (a rebase can conflict where a merge doesn't).
            Ok(MergeOutcome::Conflicts(conflicted_files)) => {
                let diff_text =
                    repository.pull_request(pull_request_id).and_then(|pull_request| repository.diff(&pull_request));
                match diff_text {
                    Ok(diff_text) => {
                        self.pull_request_diff =
                            pull_request_detail::diff_state(&diff_text, self.saved_diff_mode(), &conflicted_files)
                    }
                    Err(error) => self.error_message = Some(error.to_string()),
                }
                if let Some(detail) = &mut self.pull_request_detail {
                    detail.merge_conflicts = conflicted_files;
                }
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    /// Closes without merging; the pull request keeps its commits and diff.
    pub fn close_shown_pull_request(&mut self) {
        let (Some(repository), Some(detail)) = (&self.repository, &self.pull_request_detail) else { return };
        self.error_message = None;
        match repository.close_pull_request(detail.pull_request.id) {
            Ok(()) => self.reload_shown_pull_request(),
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    pub fn start_editing_shown_pull_request(&mut self) {
        let Some(detail) = &mut self.pull_request_detail else { return };
        self.draft = MarkdownDraft::with_text(&detail.pull_request.title, &detail.pull_request.description);
        detail.is_editing = true;
        detail.selected_tab = pull_request_detail::OVERVIEW_TAB;
        self.error_message = None;
    }

    pub fn cancel_editing_shown_pull_request(&mut self) {
        if let Some(detail) = &mut self.pull_request_detail {
            detail.is_editing = false;
        }
        self.error_message = None;
    }

    pub fn save_shown_pull_request_edit(&mut self) {
        let (Some(repository), Some(detail)) = (&self.repository, &self.pull_request_detail) else { return };
        match repository.edit_pull_request(detail.pull_request.id, &self.draft.title.text, &self.draft.description.text)
        {
            Ok(()) => {
                self.error_message = None;
                self.reload_shown_pull_request();
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    /// Re-reads the shown pull request after it changed, keeping the selected tab.
    fn reload_shown_pull_request(&mut self) {
        let Some(detail) = &self.pull_request_detail else { return };
        if let Err(error) = self.load_pull_request(detail.pull_request.id, detail.selected_tab) {
            self.error_message = Some(error.to_string());
        }
    }

    fn saved_diff_mode(&self) -> DiffMode {
        let saved_mode = self
            .project_store
            .as_ref()
            .and_then(|project_store| project_store.setting(DIFF_MODE_SETTING).ok().flatten());
        if saved_mode.as_deref() == Some("split") { DiffMode::Split } else { DiffMode::Unified }
    }

    /// Switches the diff layout and remembers it for next time.
    pub fn set_diff_mode(&mut self, diff_mode: DiffMode) {
        self.pull_request_diff.set_mode(diff_mode);
        let mode_name = if diff_mode == DiffMode::Split { "split" } else { "unified" };
        if let Some(project_store) = &self.project_store
            && let Err(error) = project_store.set_setting(DIFF_MODE_SETTING, mode_name)
        {
            self.error_message = Some(error.to_string());
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

pub fn view(app: &Dialectic, cx: &mut Cx<Dialectic>) -> El {
    let k = kit(cx);
    let mut overlay_layer = Vec::new();
    let screen = match app.screen {
        Screen::Welcome => welcome::view(app, &k),
        Screen::PullRequestList => pull_request_list::view(app, &k),
        Screen::NewPullRequest => new_pull_request::view(app, &k, &mut overlay_layer),
        Screen::PullRequestDetail => match &app.pull_request_detail {
            Some(detail) => pull_request_detail::view(app, detail, &k),
            None => pull_request_list::view(app, &k),
        },
    };
    // The window draws no title bar of its own (`App::titlebar`): the repository header is the band, with
    // the traffic lights (macOS) or caption buttons (Windows) over its ends; the welcome screen drags whole.
    let insets = cx.chrome_insets();
    let mut window_content = col().w_full().h_full().fill(k.c.bg_base);
    if app.screen != Screen::Welcome {
        window_content = window_content.child(repository_header(app, &k, insets));
    }
    rune_kit::root(window_content.child(screen), overlay_layer)
}

const TITLEBAR_HEIGHT: f32 = 52.0;

/// The bar over every repository screen: one labelled back button (to the pull request list, or from
/// the list to the repositories), then the repository's name and path.
fn repository_header(app: &Dialectic, k: &Kit, insets: rune::ChromeInsets) -> El {
    let (repository_name, repository_path) = app
        .repository
        .as_ref()
        .map(|repository| (repository.name(), repository.top_level_path.display().to_string()))
        .unwrap_or_default();
    let back_button = if app.screen == Screen::PullRequestList {
        Button::new("← Repositories")
            .variant(ButtonVariant::Ghost)
            .help("Back to the list of repositories")
            .on_press(|app: &mut Dialectic| app.show_welcome())
            .build(k)
            .key("back-to-repositories")
    } else {
        Button::new("← All pull requests")
            .variant(ButtonVariant::Ghost)
            .help("Back to this repository's pull requests")
            .on_press(|app: &mut Dialectic| app.show_pull_request_list())
            .build(k)
            .key("back-to-pull-requests")
    };
    row()
        .key("repository-header")
        .w_full()
        .h(TITLEBAR_HEIGHT)
        .window_drag()
        .pl(insets.leading + 16.0)
        .pr(insets.trailing + 16.0)
        .gap(12.0)
        .items_center()
        .fill(k.c.layer_01)
        .border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_weak)
        .child(back_button)
        .child(col().w(1.0).h(20.0).fill(k.c.border_weak))
        .child(text(repository_name).key("repository-name").font(k.ty.body_medium).color(k.c.text_strong))
        .child(text(repository_path).font(k.ty.caption()).color(k.c.text_faint))
}

/// A repository screen's body (under the header): a scrolling, centred column at most 1012 points wide, as GitHub's.
pub fn page(content: El) -> El {
    col().w_full().grow(1.0).child(
        col()
            .key("page-scroll")
            .w_full()
            .grow(1.0)
            .scroll(rune::ScrollSpec { stick_bottom: false })
            .child(row().w_full().justify_center().padding(24.0).child(col().w_full().max_w(1012.0).child(content))),
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
        let underline =
            if is_selected { rune::color::hex("#fd8c73").unwrap_or(k.c.text_accent) } else { k.c.bg_base.alpha(0.0) };
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

/// A red circle with "!": the warning sign next to anything about conflicts.
pub fn warning_mark(k: &Kit) -> El {
    col()
        .w(18.0)
        .h(18.0)
        .radius(999.0)
        .center()
        .fill(k.c.danger.fg)
        .child(text("!").font(k.ty.ui_medium).color(k.c.white))
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
type Scene = (&'static str, fn(&mut Dialectic));

/// Shows `--pull-request <id>` if given, else the newest open pull request (else the newest), on `tab`.
fn show_scene_pull_request(app: &mut Dialectic, tab: usize) {
    app.show_pull_request_list();
    let extra_args = rune::extra_args();
    let requested_id = extra_args
        .iter()
        .position(|argument| argument == "--pull-request")
        .and_then(|flag_index| extra_args.get(flag_index + 1))
        .and_then(|id_text| id_text.parse::<i64>().ok());
    let newest_open = app.pull_requests.iter().find(|pull_request| pull_request.status == PullRequestStatus::Open);
    let shown_id = requested_id.or(newest_open.or(app.pull_requests.first()).map(|pull_request| pull_request.id));
    if let Some(pull_request_id) = shown_id {
        app.show_pull_request(pull_request_id);
        if let Some(detail) = &mut app.pull_request_detail {
            detail.selected_tab = tab;
        }
    }
}

const SCENES: &[Scene] = &[
    ("pull-request-list", Dialectic::show_pull_request_list),
    ("pull-request-overview", |app| show_scene_pull_request(app, pull_request_detail::OVERVIEW_TAB)),
    ("pull-request-commits", |app| show_scene_pull_request(app, pull_request_detail::COMMITS_TAB)),
    // Point these at a pull request whose branches conflict (`--pull-request <id>`).
    ("pull-request-conflicts", |app| show_scene_pull_request(app, pull_request_detail::OVERVIEW_TAB)),
    ("pull-request-conflicts-files", |app| show_scene_pull_request(app, pull_request_detail::FILES_CHANGED_TAB)),
    ("pull-request-rebase", |app| {
        show_scene_pull_request(app, pull_request_detail::OVERVIEW_TAB);
        app.merge_method = MergeMethod::Rebase;
    }),
    ("pull-request-edit", |app| {
        show_scene_pull_request(app, pull_request_detail::OVERVIEW_TAB);
        app.start_editing_shown_pull_request();
    }),
    ("pull-request-files", |app| {
        show_scene_pull_request(app, pull_request_detail::FILES_CHANGED_TAB);
        app.pull_request_diff.set_mode(DiffMode::Unified);
    }),
    ("pull-request-files-split", |app| {
        show_scene_pull_request(app, pull_request_detail::FILES_CHANGED_TAB);
        app.pull_request_diff.set_mode(DiffMode::Split);
    }),
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

/// The diff's own keys while it shows: j/k between hunks, ]/[ between files.
fn on_key(app: &mut Dialectic, key_event: &KeyEvent, tx: &mut Tx) -> bool {
    let showing_files = app.screen == Screen::PullRequestDetail
        && app
            .pull_request_detail
            .as_ref()
            .is_some_and(|detail| detail.selected_tab == pull_request_detail::FILES_CHANGED_TAB);
    showing_files && app.pull_request_diff.key(key_event, tx)
}

pub fn run() {
    let spec = AppSpec {
        scenes: SCENES,
        on_key: Some(on_key),
        families: ("Inter", "ui-monospace, Menlo, Inter"),
        ..AppSpec::new("Dialectic", (1100.0, 760.0), Dialectic::from_command_line, view)
    };
    App::new(spec).modifier_keys().titlebar(rune::Titlebar::new(TITLEBAR_HEIGHT)).run();
}
