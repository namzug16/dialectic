//! Thin wrappers over the `git` CLI, so merges and conflicts behave exactly as they do in a terminal.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_git(working_dir: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(working_dir)
        .args(arguments)
        .output()
        .map_err(|error| Error(format!("could not run git (is it installed and on PATH?): {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
        return Err(Error(format!("git {} failed: {message}", arguments.join(" "))));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub struct RepositoryLocation {
    pub top_level_path: PathBuf,
    /// The `.git` directory every worktree of the repository shares.
    pub git_common_dir: PathBuf,
}

pub fn locate_repository(any_path_inside: &Path) -> Result<RepositoryLocation> {
    let output =
        run_git(any_path_inside, &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-common-dir"])
            .map_err(|_| Error(format!("{} is not inside a git repository", any_path_inside.display())))?;
    let mut lines = output.lines();
    match (lines.next(), lines.next()) {
        (Some(top_level), Some(common_dir)) => Ok(RepositoryLocation {
            top_level_path: PathBuf::from(top_level),
            git_common_dir: PathBuf::from(common_dir),
        }),
        _ => Err(Error(format!("{} is a bare repository; open a checkout instead", any_path_inside.display()))),
    }
}

pub fn local_branches(working_dir: &Path) -> Result<Vec<String>> {
    let output = run_git(working_dir, &["for-each-ref", "--format=%(refname:short)", "refs/heads"])?;
    Ok(output.lines().map(str::to_owned).collect())
}

/// `None` when HEAD is detached.
pub fn current_branch(working_dir: &Path) -> Option<String> {
    run_git(working_dir, &["symbolic-ref", "--short", "-q", "HEAD"]).ok().map(|branch| branch.trim().to_owned())
}

pub fn user_name(working_dir: &Path) -> String {
    run_git(working_dir, &["config", "user.name"])
        .map(|name| name.trim().to_owned())
        .unwrap_or_else(|_| "unknown".into())
}

pub fn resolve_commit_sha(working_dir: &Path, revision: &str) -> Result<String> {
    Ok(run_git(working_dir, &["rev-parse", "--verify", "--quiet", &format!("{revision}^{{commit}}")])
        .map_err(|_| Error(format!("'{revision}' is not a branch or commit in this repository")))?
        .trim()
        .to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub subject: String,
    pub author: String,
    /// `YYYY-MM-DD`
    pub authored_date: String,
}

/// Commits reachable from `head` but not from `base`, oldest first (as GitHub lists a pull request's commits).
pub fn commits_between(working_dir: &Path, base: &str, head: &str) -> Result<Vec<CommitSummary>> {
    let output = run_git(
        working_dir,
        &[
            "log",
            "--reverse",
            "--date=short",
            "--format=%H%x1f%h%x1f%s%x1f%an%x1f%ad",
            &format!("{base}..{head}"),
            "--",
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\u{1f}');
            Some(CommitSummary {
                sha: fields.next()?.to_owned(),
                short_sha: fields.next()?.to_owned(),
                subject: fields.next()?.to_owned(),
                author: fields.next()?.to_owned(),
                authored_date: fields.next()?.to_owned(),
            })
        })
        .collect())
}

/// What `head` changes since it forked from `base` (three dots: from their merge base), as GitHub shows it.
pub fn diff_since_merge_base(working_dir: &Path, base: &str, head: &str) -> Result<String> {
    run_git(working_dir, &["diff", "--no-color", "--no-ext-diff", "-M", &format!("{base}...{head}"), "--"])
}

/// Each worktree's path and the branch it has checked out (`None` when detached).
pub fn worktrees(working_dir: &Path) -> Result<Vec<(PathBuf, Option<String>)>> {
    let output = run_git(working_dir, &["worktree", "list", "--porcelain"])?;
    let mut worktrees = Vec::new();
    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            worktrees.push((PathBuf::from(path), None));
        } else if let Some(branch_ref) = line.strip_prefix("branch ")
            && let Some((_, checked_out_branch)) = worktrees.last_mut()
        {
            *checked_out_branch = Some(branch_ref.trim_start_matches("refs/heads/").to_owned());
        }
    }
    Ok(worktrees)
}

/// Tracked changes only: untracked files don't stop a merge unless it would overwrite them, which git reports itself.
pub fn has_uncommitted_changes(working_dir: &Path) -> Result<bool> {
    Ok(!run_git(working_dir, &["status", "--porcelain", "--untracked-files=no"])?.trim().is_empty())
}

pub fn conflicted_files(working_dir: &Path) -> Result<Vec<String>> {
    let output = run_git(working_dir, &["diff", "--name-only", "--diff-filter=U"])?;
    Ok(output.lines().map(str::to_owned).collect())
}
