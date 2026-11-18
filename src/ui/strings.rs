//! Every label the app shows, in each language it speaks. Adding a label means adding it to `Strings` and
//! to every language below; the compiler refuses a language that misses one.
//! Errors from the core (git, the database) stay in English: the CLI shares them.

use crate::db::{MergeMethod, PullRequestStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
}

impl Language {
    /// In picker order; each language is named in itself.
    pub const ALL: [Language; 2] = [Language::English, Language::Spanish];

    pub fn native_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
        }
    }

    /// The value saved in the settings table.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
        }
    }

    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL.into_iter().find(|language| language.code() == code)
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Language::English => &ENGLISH,
            Language::Spanish => &SPANISH,
        }
    }
}

pub struct Strings {
    pub language: &'static str,
    pub saved_projects_unavailable: fn(&str) -> String,
    pub choose_both_branches: &'static str,

    // Repository header
    pub back_to_repositories: &'static str,
    pub back_to_repositories_help: &'static str,
    pub back_to_pull_requests: &'static str,
    pub back_to_pull_requests_help: &'static str,
    pub your_changes: &'static str,
    pub your_changes_help: &'static str,
    pub pull_requests: &'static str,
    pub pull_requests_to_review: fn(usize) -> &'static str,
    pub status_open: &'static str,
    pub status_merged: &'static str,
    pub status_closed: &'static str,

    // Welcome
    pub open_repository: &'static str,
    pub open_repository_dialog_title: &'static str,
    pub folder_picker_failed: fn(&str) -> String,
    pub version: fn(&str) -> String,
    pub tagline: &'static str,
    pub no_repositories_yet: &'static str,
    pub recent_repositories: &'static str,
    pub missing: &'static str,
    pub remove: &'static str,
    pub open_path: fn(&str) -> String,

    // Settings
    pub settings: &'static str,
    pub about: &'static str,
    pub source_code: &'static str,
    pub open_link_failed: fn(&str) -> String,

    // Pull request list
    pub new_pull_request: &'static str,
    pub open_count: fn(usize) -> String,
    pub merged_or_closed_count: fn(usize) -> String,
    pub no_pull_requests_yet: &'static str,
    pub no_pull_requests_hint: &'static str,
    /// `(id, status, date, author)` under a pull request's title.
    pub pull_request_byline: fn(i64, PullRequestStatus, &str, &str) -> String,

    // New pull request and the Markdown editor
    pub open_a_pull_request: &'static str,
    pub new_pull_request_hint: &'static str,
    pub source_branch: &'static str,
    pub target_branch: &'static str,
    pub choose_a_branch: &'static str,
    pub merge: &'static str,
    pub into: &'static str,
    pub cancel: &'static str,
    pub create_pull_request: &'static str,
    pub title: &'static str,
    pub write: &'static str,
    pub preview: &'static str,
    pub nothing_to_preview: &'static str,
    pub description: &'static str,
    pub description_placeholder: &'static str,

    // Your changes
    pub refresh: &'static str,
    pub refresh_help: &'static str,
    pub current_branch: &'static str,
    pub detached_head: &'static str,
    pub new_branch_name: &'static str,
    pub new_branch_placeholder: &'static str,
    pub create_branch: &'static str,
    pub create_branch_help: &'static str,
    pub default_branch_warning: &'static str,
    pub commit_message: &'static str,
    pub commit_message_placeholder: &'static str,
    pub commit_files: fn(usize) -> String,
    pub no_uncommitted_changes: &'static str,
    pub no_uncommitted_changes_hint: &'static str,
    pub pull_requests_waiting: fn(usize) -> String,

    // Pull request detail
    pub overview_tab: &'static str,
    pub commits_tab: fn(usize) -> String,
    /// `(changed files, conflicting files)`.
    pub files_changed_tab: fn(usize, usize) -> String,
    pub edit: &'static str,
    pub save: &'static str,
    /// `(source, target, conflicting files)`.
    pub conflict_banner: fn(&str, &str, usize) -> String,
    /// `(status, author, commits)`, followed by the `source → target` chips.
    pub activity_sentence: fn(PullRequestStatus, &str, usize) -> String,
    pub no_description: &'static str,
    pub opened_this_on: fn(&str) -> String,
    /// `(how it merged, target, date)`.
    pub merged_into_on: fn(Option<MergeMethod>, &str, &str) -> String,
    pub closed_without_merging_on: fn(&str) -> String,
    pub unknown_date: &'static str,
    pub merging_blocked: &'static str,
    pub conflicts_explanation: fn(&str, &str) -> String,
    pub what_to_do: &'static str,
    /// `(author, source)`.
    pub contact_branch_owner: fn(&str, &str) -> String,
    pub agent_prompt_intro: &'static str,
    /// `(source, target, conflicting files joined)`.
    pub agent_prompt: fn(&str, &str, &str) -> String,
    pub ready_to_merge: &'static str,
    /// `(method, source, target)`.
    pub merge_explanation: fn(MergeMethod, &str, &str) -> String,
    pub merge_pull_request: &'static str,
    pub rebase_and_merge: &'static str,
    pub create_a_merge_commit: &'static str,
    pub merge_method: &'static str,
    pub close_pull_request: &'static str,
    pub no_commits: &'static str,
    pub committed_on: fn(&str, &str) -> String,

    // Files changed
    pub files_changed_count: fn(usize) -> String,
    pub unified: &'static str,
    pub split: &'static str,
    pub diff_layout: &'static str,
    pub show_file: fn(&str, bool) -> String,
    pub no_changes: &'static str,
    pub conflict: &'static str,
}

fn files_en(count: usize) -> String {
    if count == 1 { "1 file".to_owned() } else { format!("{count} files") }
}

fn files_es(count: usize) -> String {
    if count == 1 { "1 archivo".to_owned() } else { format!("{count} archivos") }
}

fn commits_en(count: usize) -> String {
    if count == 1 { "1 commit".to_owned() } else { format!("{count} commits") }
}

pub static ENGLISH: Strings = Strings {
    language: "Language",
    saved_projects_unavailable: |error| format!("Saved projects are unavailable: {error}"),
    choose_both_branches: "Choose both a source and a target branch.",

    back_to_repositories: "Repositories",
    back_to_repositories_help: "Back to the list of repositories",
    back_to_pull_requests: "All pull requests",
    back_to_pull_requests_help: "Back to this repository's pull requests",
    your_changes: "Your changes",
    your_changes_help: "See, commit and branch your uncommitted changes",
    pull_requests: "Pull requests",
    pull_requests_to_review: |count| if count == 1 { "Pull request to review" } else { "Pull requests to review" },
    status_open: "Open",
    status_merged: "Merged",
    status_closed: "Closed",

    open_repository: "Open Repository…",
    open_repository_dialog_title: "Open a Git repository",
    folder_picker_failed: |error| format!("Could not show the folder picker: {error}"),
    version: |version| format!("Version {version}"),
    tagline: "Local pull requests between the branches of one repository.",
    no_repositories_yet: "No repositories yet. Open one to start.",
    recent_repositories: "Recent repositories",
    missing: "Missing",
    remove: "Remove",
    open_path: |path| format!("Open {path}"),
    settings: "Settings",
    about: "About",
    source_code: "Source code",
    open_link_failed: |error| format!("Couldn't open the link: {error}"),

    new_pull_request: "New pull request",
    open_count: |count| format!("{count} Open"),
    merged_or_closed_count: |count| format!("{count} Merged or closed"),
    no_pull_requests_yet: "No pull requests yet",
    no_pull_requests_hint: "Open one from a branch, here or with `dialectic pr create`.",
    pull_request_byline: |id, status, date, author| {
        let verb = match status {
            PullRequestStatus::Open => "opened",
            PullRequestStatus::Merged => "merged",
            PullRequestStatus::Closed => "closed",
        };
        format!("#{id} {verb} {date} UTC by {author}")
    },

    open_a_pull_request: "Open a pull request",
    new_pull_request_hint: "Pick the branch with the changes and the branch they should merge into.",
    source_branch: "Source branch",
    target_branch: "Target branch",
    choose_a_branch: "Choose a branch",
    merge: "Merge",
    into: "→ into",
    cancel: "Cancel",
    create_pull_request: "Create pull request",
    title: "Title",
    write: "Write",
    preview: "Preview",
    nothing_to_preview: "Nothing to preview",
    description: "Description",
    description_placeholder: "Add a description. Markdown is supported: # headings, - lists, - [ ] tasks, [links](url), `code` and ``` fences.",

    refresh: "Refresh",
    refresh_help: "Read the files again after editing them",
    current_branch: "Current branch",
    detached_head: "No branch (detached)",
    new_branch_name: "New branch name",
    new_branch_placeholder: "my-new-branch",
    create_branch: "Create branch",
    create_branch_help: "Your uncommitted changes come along to the new branch",
    default_branch_warning: "You're on the main branch. Start a new branch for this work first, so it can be reviewed \
                             in a pull request before it reaches main.",
    commit_message: "Commit message",
    commit_message_placeholder: "Describe what you changed, e.g. Fix the typo on the sign-up page",
    commit_files: |count| format!("Commit {}", files_en(count)),
    no_uncommitted_changes: "No uncommitted changes",
    no_uncommitted_changes_hint: "Edit files in this repository and they'll show up here, ready to commit.",
    pull_requests_waiting: |count| {
        if count == 1 {
            "1 pull request is waiting for review".to_owned()
        } else {
            format!("{count} pull requests are waiting for review")
        }
    },

    overview_tab: "Overview",
    commits_tab: |count| format!("Commits {count}"),
    files_changed_tab: |changed, conflicting| {
        if conflicting == 0 {
            format!("Files changed {changed}")
        } else {
            format!("Files changed {changed} · {conflicting} conflicting")
        }
    },
    edit: "Edit",
    save: "Save",
    conflict_banner: |source, target, count| {
        format!(
            "{source} conflicts with {target} in {}. Merging is blocked until the conflicts are resolved.",
            files_en(count)
        )
    },
    // Only the author is recorded, so a merged pull request doesn't name who merged it.
    activity_sentence: |status, author, count| match status {
        PullRequestStatus::Open => format!("{author} wants to merge {}:", commits_en(count)),
        PullRequestStatus::Merged => format!("{} by {author} merged:", commits_en(count)),
        PullRequestStatus::Closed => format!("{} by {author}, closed without merging:", commits_en(count)),
    },
    no_description: "No description provided.",
    opened_this_on: |date| format!(" opened this on {date} UTC"),
    merged_into_on: |method, target, date| {
        let how = if method == Some(MergeMethod::Rebase) { "Rebased and merged" } else { "Merged" };
        format!("{how} into {target} on {date} UTC")
    },
    closed_without_merging_on: |date| format!("Closed without merging on {date} UTC"),
    unknown_date: "an unknown date",
    merging_blocked: "Merging is blocked: this branch has conflicts",
    conflicts_explanation: |source, target| {
        format!(
            "{source} and {target} both changed the same lines in these files, so git can't combine them on its own:"
        )
    },
    what_to_do: "What to do",
    contact_branch_owner: |author, source| {
        format!(
            "Contact {author}, who owns {source}, and ask them to fix the conflicts on their branch. Open this pull \
             request again once they have committed the fix."
        )
    },
    agent_prompt_intro: "If you think a coding agent can fix them, ask one to with this prompt:",
    agent_prompt: |source, target, files| {
        format!(
            "On the branch {source}, merge {target} and resolve the conflicts in {files}. Keep what both sides \
             intended, run the tests, then commit the merge."
        )
    },
    ready_to_merge: "Ready to merge",
    merge_explanation: |method, source, target| {
        let how = match method {
            MergeMethod::MergeCommit => {
                format!("Runs git merge --no-ff {source} into {target}: its commits plus a merge commit.")
            }
            MergeMethod::Rebase => format!(
                "Replays {source}'s commits on top of {target} and fast-forwards {target}: a linear history with no \
                 merge commit. {source} itself is left as it is."
            ),
        };
        format!("{how} If git reports conflicts, nothing changes and the files are listed here.")
    },
    merge_pull_request: "Merge pull request",
    rebase_and_merge: "Rebase and merge",
    create_a_merge_commit: "Create a merge commit",
    merge_method: "Merge method",
    close_pull_request: "Close pull request",
    no_commits: "No commits.",
    committed_on: |author, date| format!("{author} committed on {date}"),

    files_changed_count: |count| format!("{count} files changed"),
    unified: "Unified",
    split: "Split",
    diff_layout: "Diff layout",
    show_file: |path, is_conflicted| {
        if is_conflicted { format!("Show {path} (conflicting)") } else { format!("Show {path}") }
    },
    no_changes: "No changes.",
    conflict: "Conflict",
};

// "Pull request" stays in English, as Spanish-speaking developers say it; it's feminine (la pull request).
pub static SPANISH: Strings = Strings {
    language: "Idioma",
    saved_projects_unavailable: |error| format!("Los proyectos guardados no están disponibles: {error}"),
    choose_both_branches: "Elige una rama de origen y una de destino.",

    back_to_repositories: "Repositorios",
    back_to_repositories_help: "Volver a la lista de repositorios",
    back_to_pull_requests: "Todas las pull requests",
    back_to_pull_requests_help: "Volver a las pull requests de este repositorio",
    your_changes: "Tus cambios",
    your_changes_help: "Revisa tus cambios sin confirmar, haz commit y crea ramas",
    pull_requests: "Pull requests",
    pull_requests_to_review: |count| if count == 1 { "Pull request por revisar" } else { "Pull requests por revisar" },
    status_open: "Abierta",
    status_merged: "Fusionada",
    status_closed: "Cerrada",

    open_repository: "Abrir repositorio…",
    open_repository_dialog_title: "Abrir un repositorio Git",
    folder_picker_failed: |error| format!("No se pudo mostrar el selector de carpetas: {error}"),
    version: |version| format!("Versión {version}"),
    tagline: "Pull requests locales entre las ramas de un repositorio.",
    no_repositories_yet: "Aún no hay repositorios. Abre uno para empezar.",
    recent_repositories: "Repositorios recientes",
    missing: "No encontrado",
    remove: "Quitar",
    open_path: |path| format!("Abrir {path}"),
    settings: "Ajustes",
    about: "Acerca de",
    source_code: "Código fuente",
    open_link_failed: |error| format!("No se pudo abrir el enlace: {error}"),

    new_pull_request: "Nueva pull request",
    open_count: |count| format!("{count} abiertas"),
    merged_or_closed_count: |count| format!("{count} fusionadas o cerradas"),
    no_pull_requests_yet: "Aún no hay pull requests",
    no_pull_requests_hint: "Abre una desde una rama, aquí o con `dialectic pr create`.",
    pull_request_byline: |id, status, date, author| {
        let verb = match status {
            PullRequestStatus::Open => "abierta",
            PullRequestStatus::Merged => "fusionada",
            PullRequestStatus::Closed => "cerrada",
        };
        format!("#{id} {verb} el {date} UTC por {author}")
    },

    open_a_pull_request: "Abrir una pull request",
    new_pull_request_hint: "Elige la rama con los cambios y la rama en la que deben fusionarse.",
    source_branch: "Rama de origen",
    target_branch: "Rama de destino",
    choose_a_branch: "Elige una rama",
    merge: "Fusionar",
    into: "→ en",
    cancel: "Cancelar",
    create_pull_request: "Crear pull request",
    title: "Título",
    write: "Escribir",
    preview: "Vista previa",
    nothing_to_preview: "Nada que previsualizar",
    description: "Descripción",
    description_placeholder: "Añade una descripción. Admite Markdown: # títulos, - listas, - [ ] tareas, [enlaces](url), `código` y bloques ```.",

    refresh: "Actualizar",
    refresh_help: "Vuelve a leer los archivos después de editarlos",
    current_branch: "Rama actual",
    detached_head: "Sin rama (HEAD separado)",
    new_branch_name: "Nombre de la nueva rama",
    new_branch_placeholder: "mi-nueva-rama",
    create_branch: "Crear rama",
    create_branch_help: "Tus cambios sin confirmar pasan a la nueva rama",
    default_branch_warning: "Estás en la rama principal. Empieza primero una rama nueva para este trabajo, así podrá \
                             revisarse en una pull request antes de llegar a main.",
    commit_message: "Mensaje del commit",
    commit_message_placeholder: "Describe qué cambiaste, p. ej. Corrige la errata en la página de registro",
    commit_files: |count| format!("Hacer commit de {}", files_es(count)),
    no_uncommitted_changes: "No hay cambios sin confirmar",
    no_uncommitted_changes_hint: "Edita archivos de este repositorio y aparecerán aquí, listos para hacer commit.",
    pull_requests_waiting: |count| {
        if count == 1 {
            "1 pull request espera revisión".to_owned()
        } else {
            format!("{count} pull requests esperan revisión")
        }
    },

    overview_tab: "Resumen",
    commits_tab: |count| format!("Commits {count}"),
    files_changed_tab: |changed, conflicting| {
        if conflicting == 0 {
            format!("Archivos modificados {changed}")
        } else {
            format!("Archivos modificados {changed} · {conflicting} en conflicto")
        }
    },
    edit: "Editar",
    save: "Guardar",
    conflict_banner: |source, target, count| {
        format!(
            "{source} tiene conflictos con {target} en {}. La fusión está bloqueada hasta que se resuelvan.",
            files_es(count)
        )
    },
    activity_sentence: |status, author, count| {
        let commits = if count == 1 { "1 commit".to_owned() } else { format!("{count} commits") };
        match status {
            PullRequestStatus::Open => format!("{author} quiere fusionar {commits}:"),
            PullRequestStatus::Merged => format!("{commits} de {author} fusionados:"),
            PullRequestStatus::Closed => format!("{commits} de {author}, cerrada sin fusionar:"),
        }
    },
    no_description: "No se proporcionó una descripción.",
    opened_this_on: |date| format!(" abrió esto el {date} UTC"),
    merged_into_on: |method, target, date| {
        let how = if method == Some(MergeMethod::Rebase) { "Rebase y fusión" } else { "Fusionada" };
        format!("{how} en {target} el {date} UTC")
    },
    closed_without_merging_on: |date| format!("Cerrada sin fusionar el {date} UTC"),
    unknown_date: "una fecha desconocida",
    merging_blocked: "La fusión está bloqueada: esta rama tiene conflictos",
    conflicts_explanation: |source, target| {
        format!(
            "{source} y {target} cambiaron las mismas líneas en estos archivos, así que git no puede combinarlas por \
             sí solo:"
        )
    },
    what_to_do: "Qué hacer",
    contact_branch_owner: |author, source| {
        format!(
            "Contacta a {author}, responsable de {source}, y pídele que resuelva los conflictos en su rama. Vuelve a \
             abrir esta pull request cuando haya hecho commit de la solución."
        )
    },
    agent_prompt_intro: "Si crees que un agente de código puede resolverlos, pídeselo con este prompt:",
    agent_prompt: |source, target, files| {
        format!(
            "En la rama {source}, fusiona {target} y resuelve los conflictos en {files}. Conserva lo que ambos lados \
             pretendían, ejecuta los tests y luego haz commit de la fusión."
        )
    },
    ready_to_merge: "Lista para fusionar",
    merge_explanation: |method, source, target| {
        let how = match method {
            MergeMethod::MergeCommit => {
                format!("Ejecuta git merge --no-ff {source} en {target}: sus commits más un commit de fusión.")
            }
            MergeMethod::Rebase => format!(
                "Reaplica los commits de {source} sobre {target} y avanza {target} (fast-forward): un historial lineal \
                 sin commit de fusión. {source} queda tal como está."
            ),
        };
        format!("{how} Si git informa de conflictos, no cambia nada y los archivos se listan aquí.")
    },
    merge_pull_request: "Fusionar pull request",
    rebase_and_merge: "Rebase y fusionar",
    create_a_merge_commit: "Crear un commit de fusión",
    merge_method: "Método de fusión",
    close_pull_request: "Cerrar pull request",
    no_commits: "No hay commits.",
    committed_on: |author, date| format!("{author} hizo commit el {date}"),

    files_changed_count: |count| format!("{count} archivos modificados"),
    unified: "Unificada",
    split: "Dividida",
    diff_layout: "Diseño del diff",
    show_file: |path, is_conflicted| {
        if is_conflicted { format!("Mostrar {path} (en conflicto)") } else { format!("Mostrar {path}") }
    },
    no_changes: "Sin cambios.",
    conflict: "Conflicto",
};
