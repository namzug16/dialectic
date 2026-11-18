//! Thin wrappers over the `git` CLI, so merges and conflicts behave exactly as they do in a terminal.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_git(working_dir: &Path, arguments: &[&str]) -> Result<String> {
    run_git_accepting_exit_codes(working_dir, arguments, &[0]).map(|(_, stdout)| stdout)
}

/// For commands whose exit code carries an answer (`git merge-tree` exits 1 on conflicts, `git diff
/// --no-index` on differences): `Ok` with the code and stdout when the code is one of `accepted_exit_codes`.
fn run_git_accepting_exit_codes(
    working_dir: &Path,
    arguments: &[&str],
    accepted_exit_codes: &[i32],
) -> Result<(i32, String)> {
    run_git_command(working_dir, arguments, accepted_exit_codes, None)
}

/// `index_file` swaps in another index (`GIT_INDEX_FILE`) for this one command.
fn run_git_command(
    working_dir: &Path,
    arguments: &[&str],
    accepted_exit_codes: &[i32],
    index_file: Option<&Path>,
) -> Result<(i32, String)> {
    let mut command = Command::new("git");
    command.arg("-C").arg(working_dir).args(arguments);
    if let Some(index_file) = index_file {
        command.env("GIT_INDEX_FILE", index_file);
    }
    let output = command
        .output()
        .map_err(|error| Error(format!("could not run git (is it installed and on PATH?): {error}")))?;
    match output.status.code() {
        Some(exit_code) if accepted_exit_codes.contains(&exit_code) => {
            Ok((exit_code, String::from_utf8_lossy(&output.stdout).into_owned()))
        }
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let message = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            Err(Error(format!("git {} failed: {message}", arguments.join(" "))))
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    /// `None` for binary files, which git doesn't count in lines.
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
}

/// Per-file line counts of the same three-dot diff.
pub fn changed_files_since_merge_base(working_dir: &Path, base: &str, head: &str) -> Result<Vec<ChangedFile>> {
    let output = run_git(working_dir, &["diff", "--numstat", "--no-renames", &format!("{base}...{head}"), "--"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let additions = fields.next()?.parse().ok();
            let deletions = fields.next()?.parse().ok();
            Some(ChangedFile { path: fields.next()?.to_owned(), additions, deletions })
        })
        .collect())
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

/// The files that would conflict if `source` were merged into `target`, found without touching any
/// checkout (`git merge-tree`, git 2.38+). Empty when they merge cleanly.
pub fn predicted_merge_conflicts(working_dir: &Path, target: &str, source: &str) -> Result<Vec<String>> {
    let (exit_code, output) = run_git_accepting_exit_codes(
        working_dir,
        &["merge-tree", "--write-tree", "--name-only", "--no-messages", target, source],
        &[0, 1],
    )?;
    if exit_code == 0 {
        return Ok(Vec::new());
    }
    // The first line is the merged tree's id; the conflicted paths follow, up to a blank line.
    Ok(output.lines().skip(1).take_while(|line| !line.is_empty()).map(str::to_owned).collect())
}

/// Everything committing would record: tracked changes against HEAD, plus untracked files as new files.
pub fn uncommitted_diff(working_dir: &Path) -> Result<String> {
    // Untracked files are marked intent-to-add in a scratch copy of the index, so one `git diff HEAD` shows
    // them as new files without staging anything for real; a `git diff --no-index` per file took seconds.
    let real_index = run_git(working_dir, &["rev-parse", "--path-format=absolute", "--git-path", "index"])?;
    let scratch_index = std::env::temp_dir().join(format!("dialectic-index-{}", std::process::id()));
    if let Err(error) = std::fs::copy(real_index.trim(), &scratch_index)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(Error(format!("could not copy the git index: {error}")));
    }
    let diff = run_git_with_index(working_dir, &scratch_index, &["add", "--intent-to-add", "--", "."]).and_then(|_| {
        run_git_with_index(working_dir, &scratch_index, &["diff", "--no-color", "--no-ext-diff", "HEAD", "--"])
    });
    let _ = std::fs::remove_file(&scratch_index);
    diff
}

fn run_git_with_index(working_dir: &Path, index_file: &Path, arguments: &[&str]) -> Result<String> {
    run_git_command(working_dir, arguments, &[0], Some(index_file)).map(|(_, stdout)| stdout)
}

/// Tracked and untracked changes: whether there is anything for `commit_all` to record.
pub fn has_changes_to_commit(working_dir: &Path) -> Result<bool> {
    Ok(!run_git(working_dir, &["status", "--porcelain"])?.trim().is_empty())
}

/// Stages every change, new and deleted files included, and commits it.
pub fn commit_all(working_dir: &Path, message: &str) -> Result<()> {
    run_git(working_dir, &["add", "--all"])?;
    run_git(working_dir, &["commit", "--quiet", "--message", message])?;
    Ok(())
}
