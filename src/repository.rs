//! One opened repository: its pull requests and the rules for creating, editing and finishing them.

use crate::db::{NewPullRequest, PullRequest, PullRequestStatus, PullRequestStore};
use crate::git::{self, CommitSummary};
use crate::{Error, Result};
use std::path::{Path, PathBuf};

pub struct Repository {
    pub top_level_path: PathBuf,
    pub git_common_dir: PathBuf,
    pull_requests: PullRequestStore,
}

impl Repository {
    pub fn open(any_path_inside: &Path) -> Result<Self> {
        let location = git::locate_repository(any_path_inside)?;
        let pull_requests = PullRequestStore::open_in_git_dir(&location.git_common_dir)?;
        Ok(Self {
            top_level_path: location.top_level_path,
            git_common_dir: location.git_common_dir,
            pull_requests,
        })
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
            return Err(Error(format!(
                "source and target are both '{source_branch}'; pick two different branches"
            )));
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
        self.pull_requests
            .update_title_and_description(pull_request.id, title, description)
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

    fn open_pull_request(&self, pull_request_id: i64) -> Result<PullRequest> {
        let pull_request = self.pull_requests.get(pull_request_id)?;
        if pull_request.status != PullRequestStatus::Open {
            return Err(Error(format!(
                "pull request #{pull_request_id} is {}",
                pull_request.status.as_str()
            )));
        }
        Ok(pull_request)
    }
}
