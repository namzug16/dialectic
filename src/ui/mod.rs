//! The desktop app: one window whose `Screen` picks what `view` draws.

mod welcome;

use crate::db::ProjectStore;
use crate::db::SavedProject;
use crate::repository::Repository;
use rune::{App, AppSpec, Cx, El, col, row, text};
use rune_kit::Kit;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    PullRequestList,
}

pub struct Dialectic {
    pub screen: Screen,
    /// `None` when the app data directory couldn't be opened; `error_message` says why.
    pub project_store: Option<ProjectStore>,
    pub saved_projects: Vec<SavedProject>,
    pub repository: Option<Repository>,
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

    pub fn show_pull_request_list(&mut self) {
        self.error_message = None;
        self.screen = Screen::PullRequestList;
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
        Screen::PullRequestList => col().child(repository_header(app, &k)),
    };
    let _ = &mut overlay_layer;
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

pub fn run() {
    let spec = AppSpec {
        ..AppSpec::new("Dialectic", (1100.0, 760.0), Dialectic::from_command_line, view)
    };
    App::new(spec).modifier_keys().run();
}
