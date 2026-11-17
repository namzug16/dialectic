//! `dialectic pr …` and `dialectic guide`: the terminal interface, written for agents as much as people.
//! Every command goes through `Repository`, so the GUI's rules apply here too.

use crate::db::{PullRequest, PullRequestStatus};
use crate::repository::Repository;
use crate::{Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const AGENT_GUIDE: &str = include_str!("guide.md");

/// Whether these arguments are for the CLI rather than the GUI (`dialectic [path]`, plus Rune's own flags).
pub fn is_cli_invocation(arguments: &[String]) -> bool {
    matches!(
        arguments.get(1).map(String::as_str),
        Some("pr" | "guide" | "help" | "--help" | "-h" | "--version" | "-V" | "-C" | "--repository")
    )
}

#[derive(Parser)]
#[command(
    name = "dialectic",
    version,
    about = "Local Git pull requests between the branches of one repository",
    long_about = "Local Git pull requests between the branches of one repository.\n\n\
        Run `dialectic [path]` to open the desktop app (on the repository at path, if given).\n\
        Agents: run `dialectic guide` first; it explains the workflow and the rules.",
    after_help = "Exit codes: 0 success, 1 the command failed (message on stderr), 2 invalid arguments."
)]
struct Cli {
    /// Repository to work in (any directory inside it, any worktree).
    #[arg(short = 'C', long = "repository", global = true, default_value = ".")]
    repository: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create, inspect, edit and close pull requests.
    #[command(subcommand)]
    Pr(PullRequestCommand),
    /// Print the guide for agents: how and when to open a pull request.
    Guide,
}

#[derive(Subcommand)]
enum PullRequestCommand {
    /// Open a pull request from the source branch into the target branch.
    Create {
        /// Branch the changes should merge into, e.g. main.
        #[arg(long)]
        target: String,
        /// Branch with the changes [default: the current branch].
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        title: String,
        #[command(flatten)]
        description: DescriptionArguments,
        /// Print the created pull request as JSON.
        #[arg(long)]
        json: bool,
    },
    /// List pull requests, open ones first.
    List {
        #[arg(long, value_enum)]
        status: Option<StatusFilter>,
        #[arg(long)]
        json: bool,
    },
    /// Show a pull request with its commits and changed files.
    Show {
        pull_request_id: i64,
        #[arg(long)]
        json: bool,
    },
    /// Print a pull request's unified diff (what the source changes since it forked from the target).
    Diff { pull_request_id: i64 },
    /// Change an open pull request's title and/or description.
    Edit {
        pull_request_id: i64,
        #[arg(long)]
        title: Option<String>,
        #[command(flatten)]
        description: DescriptionArguments,
        #[arg(long)]
        json: bool,
    },
    /// Close an open pull request without merging it.
    Close { pull_request_id: i64 },
}

#[derive(clap::Args)]
struct DescriptionArguments {
    /// Markdown description.
    #[arg(long, conflicts_with = "description_file")]
    description: Option<String>,
    /// Read the Markdown description from a file, or from stdin with `-`.
    #[arg(long, value_name = "PATH")]
    description_file: Option<PathBuf>,
}

impl DescriptionArguments {
    fn read(&self) -> Result<Option<String>> {
        match (&self.description, &self.description_file) {
            (Some(description), _) => Ok(Some(description.clone())),
            (None, Some(path)) if path == Path::new("-") => {
                let mut description = String::new();
                std::io::stdin().read_to_string(&mut description)?;
                Ok(Some(description))
            }
            (None, Some(path)) => std::fs::read_to_string(path)
                .map(Some)
                .map_err(|error| Error(format!("could not read {}: {error}", path.display()))),
            (None, None) => Ok(None),
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum StatusFilter {
    Open,
    Merged,
    Closed,
}

/// Runs the CLI and returns the process exit code. Clap exits by itself (code 2) on invalid arguments.
pub fn run(arguments: Vec<String>) -> i32 {
    let cli = Cli::parse_from(arguments);
    match execute(cli) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    }
}

fn execute(cli: Cli) -> Result<String> {
    let pull_request_command = match cli.command {
        Command::Guide => return Ok(AGENT_GUIDE.to_owned()),
        Command::Pr(pull_request_command) => pull_request_command,
    };
    let repository = Repository::open(&cli.repository)?;
    match pull_request_command {
        PullRequestCommand::Create { target, source, title, description, json } => {
            let source = match source {
                Some(source) => source,
                None => repository.current_branch().ok_or_else(|| {
                    Error("HEAD is detached; pass --source <branch> to say which branch to propose".into())
                })?,
            };
            let description = description.read()?.unwrap_or_default();
            let pull_request_id = repository.create_pull_request(&title, &description, &source, &target)?;
            let pull_request = repository.pull_request(pull_request_id)?;
            if json {
                return Ok(json_line(&pull_request_json(&pull_request)));
            }
            Ok(format!(
                "Created pull request #{pull_request_id}: {}\n  {} → {}\nA human reviews and merges it in the Dialectic app.\n",
                pull_request.title, pull_request.source_branch, pull_request.target_branch
            ))
        }
        PullRequestCommand::List { status, json } => {
            let wanted_status = status.map(|status| match status {
                StatusFilter::Open => PullRequestStatus::Open,
                StatusFilter::Merged => PullRequestStatus::Merged,
                StatusFilter::Closed => PullRequestStatus::Closed,
            });
            let pull_requests: Vec<PullRequest> = repository
                .list_pull_requests()?
                .into_iter()
                .filter(|pull_request| wanted_status.is_none_or(|wanted| pull_request.status == wanted))
                .collect();
            if json {
                return Ok(json_line(&Value::Array(pull_requests.iter().map(pull_request_json).collect())));
            }
            if pull_requests.is_empty() {
                return Ok("No pull requests.\n".into());
            }
            Ok(pull_requests
                .iter()
                .map(|pull_request| {
                    format!(
                        "#{:<4} {:<7} {}  ({} → {})\n",
                        pull_request.id,
                        pull_request.status.as_str(),
                        pull_request.title,
                        pull_request.source_branch,
                        pull_request.target_branch
                    )
                })
                .collect())
        }
        PullRequestCommand::Show { pull_request_id, json } => {
            let pull_request = repository.pull_request(pull_request_id)?;
            let commits = repository.commits(&pull_request)?;
            let changed_files = repository.changed_files(&pull_request)?;
            if json {
                let mut shown = pull_request_json(&pull_request);
                shown["commits"] = commits
                    .iter()
                    .map(|commit| {
                        json!({"sha": commit.sha, "subject": commit.subject, "author": commit.author, "date": commit.authored_date})
                    })
                    .collect();
                shown["changed_files"] = changed_files
                    .iter()
                    .map(|file| json!({"path": file.path, "additions": file.additions, "deletions": file.deletions}))
                    .collect();
                return Ok(json_line(&shown));
            }
            let mut shown = format!(
                "#{} {}\n{} · {} → {} · by {} · opened {} UTC\n\n{}\n\nCommits ({}):\n",
                pull_request.id,
                pull_request.title,
                pull_request.status.as_str(),
                pull_request.source_branch,
                pull_request.target_branch,
                pull_request.author,
                pull_request.created_at,
                if pull_request.description.trim().is_empty() {
                    "(no description)"
                } else {
                    pull_request.description.trim()
                },
                commits.len()
            );
            for commit in &commits {
                shown.push_str(&format!("  {} {}\n", commit.short_sha, commit.subject));
            }
            shown.push_str(&format!("\nChanged files ({}):\n", changed_files.len()));
            for file in &changed_files {
                let counts = match (file.additions, file.deletions) {
                    (Some(additions), Some(deletions)) => format!("+{additions} -{deletions}"),
                    _ => "binary".into(),
                };
                shown.push_str(&format!("  {counts:>10}  {}\n", file.path));
            }
            Ok(shown)
        }
        PullRequestCommand::Diff { pull_request_id } => repository.diff(&repository.pull_request(pull_request_id)?),
        PullRequestCommand::Edit { pull_request_id, title, description, json } => {
            let current = repository.pull_request(pull_request_id)?;
            let new_description = description.read()?;
            if title.is_none() && new_description.is_none() {
                return Err(Error("nothing to change; pass --title and/or --description/--description-file".into()));
            }
            repository.edit_pull_request(
                pull_request_id,
                title.as_deref().unwrap_or(&current.title),
                new_description.as_deref().unwrap_or(&current.description),
            )?;
            let edited = repository.pull_request(pull_request_id)?;
            if json {
                return Ok(json_line(&pull_request_json(&edited)));
            }
            Ok(format!("Updated pull request #{pull_request_id}: {}\n", edited.title))
        }
        PullRequestCommand::Close { pull_request_id } => {
            repository.close_pull_request(pull_request_id)?;
            Ok(format!("Closed pull request #{pull_request_id} without merging.\n"))
        }
    }
}

fn pull_request_json(pull_request: &PullRequest) -> Value {
    json!({
        "id": pull_request.id,
        "title": pull_request.title,
        "description": pull_request.description,
        "source_branch": pull_request.source_branch,
        "target_branch": pull_request.target_branch,
        "author": pull_request.author,
        "status": pull_request.status.as_str(),
        "created_at": pull_request.created_at,
        "merged_or_closed_at": pull_request.merged_or_closed_at,
    })
}

fn json_line(value: &Value) -> String {
    format!("{value}\n")
}
