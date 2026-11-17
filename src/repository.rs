//! One opened repository: its pull requests and the rules for creating, editing and finishing them.

use crate::db::{MergeMethod, NewPullRequest, PullRequest, PullRequestStatus, PullRequestStore};
use crate::git::{self, ChangedFile, CommitSummary};
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Repository {
    pub top_level_path: PathBuf,
    pub git_common_dir: PathBuf,
    pull_requests: PullRequestStore,
}

impl Repository {
    pub fn open(any_path_inside: &Path) -> Result<Self> {
        let location = git::locate_repository(any_path_inside)?;
        let pull_requests = PullRequestStore::open_in_git_dir(&location.git_common_dir)?;
        Ok(Self { top_level_path: location.top_level_path, git_common_dir: location.git_common_dir, pull_requests })
    }

    pub fn name(&self) -> String {
        self.top_level_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.top_level_path.display().to_string())
    }

    pub fn local_branches(&self) -> Result<Vec<String>> {
        git::local_branches(&self.top_level_path)
    }

    pub fn current_branch(&self) -> Option<String> {
        git::current_branch(&self.top_level_path)
    }

    pub fn create_pull_request(
        &self,
        title: &str,
        description: &str,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<i64> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error("a pull request needs a title".into()));
        }
        if source_branch == target_branch {
            return Err(Error(format!("source and target are both '{source_branch}'; pick two different branches")));
        }
        let branches = self.local_branches()?;
        for branch in [source_branch, target_branch] {
            if !branches.iter().any(|existing| existing == branch) {
                return Err(Error(format!("'{branch}' is not a local branch")));
            }
        }
        if let Some(existing_id) = self.pull_requests.find_open_between(source_branch, target_branch)? {
            return Err(Error(format!(
                "pull request #{existing_id} is already open from '{source_branch}' into '{target_branch}'"
            )));
        }
        if git::commits_between(&self.top_level_path, target_branch, source_branch)?.is_empty() {
            return Err(Error(format!(
                "'{source_branch}' has no commits that '{target_branch}' doesn't already have; commit something first"
            )));
        }
        self.pull_requests.create(&NewPullRequest {
            title,
            description,
            source_branch,
            target_branch,
            author: &git::user_name(&self.top_level_path),
        })
    }

    pub fn list_pull_requests(&self) -> Result<Vec<PullRequest>> {
        self.pull_requests.list()
    }

    pub fn pull_request(&self, pull_request_id: i64) -> Result<PullRequest> {
        self.pull_requests.get(pull_request_id)
    }

    pub fn edit_pull_request(&self, pull_request_id: i64, title: &str, description: &str) -> Result<()> {
        let pull_request = self.open_pull_request(pull_request_id)?;
        let title = title.trim();
        if title.is_empty() {
            return Err(Error("a pull request needs a title".into()));
        }
        self.pull_requests.update_title_and_description(pull_request.id, title, description)
    }

    /// The (base, head) revisions a pull request compares: live branches while open, frozen commits after.
    fn compared_revisions(pull_request: &PullRequest) -> (String, String) {
        match (&pull_request.frozen_target_sha, &pull_request.frozen_source_sha) {
            (Some(target_sha), Some(source_sha)) => (target_sha.clone(), source_sha.clone()),
            _ => (pull_request.target_branch.clone(), pull_request.source_branch.clone()),
        }
    }

    pub fn commits(&self, pull_request: &PullRequest) -> Result<Vec<CommitSummary>> {
        let (base, head) = Self::compared_revisions(pull_request);
        git::commits_between(&self.top_level_path, &base, &head)
    }

    /// Unified `git diff` text, for `rune_kit::diff::parse` or a terminal.
    pub fn diff(&self, pull_request: &PullRequest) -> Result<String> {
        let (base, head) = Self::compared_revisions(pull_request);
        git::diff_since_merge_base(&self.top_level_path, &base, &head)
    }

    pub fn changed_files(&self, pull_request: &PullRequest) -> Result<Vec<ChangedFile>> {
        let (base, head) = Self::compared_revisions(pull_request);
        git::changed_files_since_merge_base(&self.top_level_path, &base, &head)
    }

    /// The files that conflict between an open pull request's branches, so the app can block the merge
    /// before anyone tries it. Always empty for merged or closed pull requests.
    pub fn merge_conflicts(&self, pull_request: &PullRequest) -> Result<Vec<String>> {
        if pull_request.status != PullRequestStatus::Open {
            return Ok(Vec::new());
        }
        git::predicted_merge_conflicts(&self.top_level_path, &pull_request.target_branch, &pull_request.source_branch)
    }

    /// Merges the source branch into the target branch with git:
    /// - `MergeCommit` runs a standard `git merge --no-ff`.
    /// - `Rebase` replays the source's commits onto the target in a temporary detached worktree (the source
    ///   branch never moves, as with GitHub's "Rebase and merge"), then fast-forwards the target to them.
    ///
    /// The target is updated in the worktree that has it checked out (refusing if it has uncommitted
    /// changes), else without touching any checkout. On conflicts git's operation is aborted, leaving the
    /// target as it was.
    pub fn merge_pull_request(&self, pull_request_id: i64, merge_method: MergeMethod) -> Result<MergeOutcome> {
        let pull_request = self.open_pull_request(pull_request_id)?;
        let target_branch = pull_request.target_branch.as_str();
        let target_sha_before_merge = git::resolve_commit_sha(&self.top_level_path, target_branch)?;
        let source_sha = git::resolve_commit_sha(&self.top_level_path, &pull_request.source_branch)?;

        let worktree_with_target = git::worktrees(&self.top_level_path)?
            .into_iter()
            .find(|(_, checked_out_branch)| checked_out_branch.as_deref() == Some(target_branch))
            .map(|(worktree_path, _)| worktree_path);
        if let Some(worktree_path) = &worktree_with_target
            && git::has_uncommitted_changes(worktree_path)?
        {
            return Err(Error(format!(
                "'{target_branch}' is checked out in {} with uncommitted changes; commit or stash them, then merge again",
                worktree_path.display()
            )));
        }

        let merge_outcome = match (merge_method, &worktree_with_target) {
            (MergeMethod::MergeCommit, Some(worktree_path)) => {
                merge_in_worktree(worktree_path, &pull_request.source_branch)?
            }
            (MergeMethod::MergeCommit, None) => {
                self.in_temporary_worktree(pull_request_id, &[target_branch], |worktree_path| {
                    merge_in_worktree(worktree_path, &pull_request.source_branch)
                })?
            }
            (MergeMethod::Rebase, _) => {
                let rebase_result =
                    self.in_temporary_worktree(pull_request_id, &["--detach", &source_sha], |worktree_path| {
                        rebase_onto(worktree_path, target_branch)
                    })?;
                match rebase_result {
                    Err(conflicted_files) => MergeOutcome::Conflicts(conflicted_files),
                    Ok(rebased_head_sha) => {
                        match &worktree_with_target {
                            Some(worktree_path) => {
                                git::run_git(worktree_path, &["merge", "--ff-only", "--quiet", &rebased_head_sha])?;
                            }
                            // The old value makes git refuse if the target moved since we read it.
                            None => {
                                git::run_git(
                                    &self.top_level_path,
                                    &[
                                        "update-ref",
                                        &format!("refs/heads/{target_branch}"),
                                        &rebased_head_sha,
                                        &target_sha_before_merge,
                                    ],
                                )?;
                            }
                        }
                        MergeOutcome::Merged
                    }
                }
            }
        };
        if merge_outcome == MergeOutcome::Merged {
            self.pull_requests.finish(
                pull_request_id,
                PullRequestStatus::Merged,
                Some(merge_method),
                &target_sha_before_merge,
                &source_sha,
            )?;
        }
        Ok(merge_outcome)
    }

    /// Runs `operation` in a new worktree made with `git worktree add <path> <checkout_arguments>`, then
    /// removes the worktree whatever the operation returned.
    fn in_temporary_worktree<T>(
        &self,
        pull_request_id: i64,
        checkout_arguments: &[&str],
        operation: impl FnOnce(&Path) -> Result<T>,
    ) -> Result<T> {
        // Unique per call: several merges (or test threads) in one process must not share a path.
        static TEMPORARY_WORKTREE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let call_number = TEMPORARY_WORKTREE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_worktree = std::env::temp_dir()
            .join(format!("dialectic-merge-{}-{pull_request_id}-{call_number}", std::process::id()));
        let temporary_worktree_text = temporary_worktree.to_string_lossy().into_owned();
        let mut add_arguments = vec!["worktree", "add", "--quiet", temporary_worktree_text.as_str()];
        add_arguments.extend_from_slice(checkout_arguments);
        git::run_git(&self.top_level_path, &add_arguments)?;
        let operation_result = operation(&temporary_worktree);
        let removal_result =
            git::run_git(&self.top_level_path, &["worktree", "remove", "--force", &temporary_worktree_text]);
        let value = operation_result?;
        removal_result?;
        Ok(value)
    }

    /// Closes without merging; its commits and diff stay viewable from the branches' current commits.
    pub fn close_pull_request(&self, pull_request_id: i64) -> Result<()> {
        let pull_request = self.open_pull_request(pull_request_id)?;
        let target_sha = git::resolve_commit_sha(&self.top_level_path, &pull_request.target_branch)?;
        let source_sha = git::resolve_commit_sha(&self.top_level_path, &pull_request.source_branch)?;
        self.pull_requests.finish(pull_request_id, PullRequestStatus::Closed, None, &target_sha, &source_sha)
    }

    fn open_pull_request(&self, pull_request_id: i64) -> Result<PullRequest> {
        let pull_request = self.pull_requests.get(pull_request_id)?;
        if pull_request.status != PullRequestStatus::Open {
            return Err(Error(format!("pull request #{pull_request_id} is {}", pull_request.status.as_str())));
        }
        Ok(pull_request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    Merged,
    /// Files git reported as conflicting; the merge was aborted and the pull request stays open.
    Conflicts(Vec<String>),
}

fn merge_in_worktree(worktree_path: &Path, source_branch: &str) -> Result<MergeOutcome> {
    match git::run_git(worktree_path, &["merge", "--no-ff", "--no-edit", source_branch]) {
        Ok(_) => Ok(MergeOutcome::Merged),
        Err(merge_error) => {
            let conflicted_files = git::conflicted_files(worktree_path)?;
            if conflicted_files.is_empty() {
                return Err(merge_error);
            }
            git::run_git(worktree_path, &["merge", "--abort"])?;
            Ok(MergeOutcome::Conflicts(conflicted_files))
        }
    }
}

/// Rebases the worktree's (detached) HEAD onto `target_branch`: `Ok` with the rebased head, or `Err` with
/// the files git reported as conflicting, after aborting the rebase.
fn rebase_onto(worktree_path: &Path, target_branch: &str) -> Result<std::result::Result<String, Vec<String>>> {
    match git::run_git(worktree_path, &["rebase", "--quiet", target_branch]) {
        Ok(_) => Ok(Ok(git::resolve_commit_sha(worktree_path, "HEAD")?)),
        Err(rebase_error) => {
            let conflicted_files = git::conflicted_files(worktree_path)?;
            if conflicted_files.is_empty() {
                return Err(rebase_error);
            }
            git::run_git(worktree_path, &["rebase", "--abort"])?;
            Ok(Err(conflicted_files))
        }
    }
}
