//! Two SQLite files:
//! - `<git common dir>/dialectic.db`: the repository's pull requests, shared by every worktree and app instance.
//! - `<app data dir>/projects.db`: this user's saved repositories and settings.

use crate::{Error, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestStatus {
    Open,
    Merged,
    Closed,
}

impl PullRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PullRequestStatus::Open => "open",
            PullRequestStatus::Merged => "merged",
            PullRequestStatus::Closed => "closed",
        }
    }

    pub fn parse(status_text: &str) -> Result<Self> {
        match status_text {
            "open" => Ok(PullRequestStatus::Open),
            "merged" => Ok(PullRequestStatus::Merged),
            "closed" => Ok(PullRequestStatus::Closed),
            other => Err(Error(format!("unknown pull request status '{other}' (expected open, merged or closed)"))),
        }
    }
}

/// How a pull request's commits reached its target, as GitHub's merge button offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    /// `git merge --no-ff`: the source's commits plus a merge commit.
    MergeCommit,
    /// The source's commits replayed onto the target, which then fast-forwards; no merge commit.
    Rebase,
}

impl MergeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            MergeMethod::MergeCommit => "merge",
            MergeMethod::Rebase => "rebase",
        }
    }

    pub fn parse(method_text: &str) -> Result<Self> {
        match method_text {
            "merge" => Ok(MergeMethod::MergeCommit),
            "rebase" => Ok(MergeMethod::Rebase),
            other => Err(Error(format!("unknown merge method '{other}' (expected merge or rebase)"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author: String,
    pub status: PullRequestStatus,
    /// UTC, `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
    pub merged_or_closed_at: Option<String>,
    /// Target and source commits frozen when the pull request merged or closed, so its commits and
    /// diff still show after the branches move on. `None` while open: those are read live.
    pub frozen_target_sha: Option<String>,
    pub frozen_source_sha: Option<String>,
    /// Set once merged.
    pub merge_method: Option<MergeMethod>,
}

pub struct NewPullRequest<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub source_branch: &'a str,
    pub target_branch: &'a str,
    pub author: &'a str,
}

fn open_shared_connection(database_path: &Path) -> Result<Connection> {
    let connection = Connection::open(database_path)?;
    // Two developers' app instances (and agents' CLI calls) share the file.
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    Ok(connection)
}

pub struct PullRequestStore {
    connection: Connection,
}

const PULL_REQUEST_COLUMNS: &str = "id, title, description, source_branch, target_branch, author, status, \
     created_at, merged_or_closed_at, frozen_target_sha, frozen_source_sha, merge_method";

impl PullRequestStore {
    pub fn open_in_git_dir(git_common_dir: &Path) -> Result<Self> {
        Self::open(&git_common_dir.join("dialectic.db"))
    }

    pub fn open(database_path: &Path) -> Result<Self> {
        let connection = open_shared_connection(database_path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS pull_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                source_branch TEXT NOT NULL,
                target_branch TEXT NOT NULL,
                author TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                merged_or_closed_at TEXT,
                frozen_target_sha TEXT,
                frozen_source_sha TEXT,
                merge_method TEXT
            );",
        )?;
        // Databases made before merge methods existed lack the column.
        let has_merge_method_column: bool = connection.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('pull_requests') WHERE name = 'merge_method'",
            [],
            |row| row.get(0),
        )?;
        if !has_merge_method_column {
            connection.execute_batch("ALTER TABLE pull_requests ADD COLUMN merge_method TEXT;")?;
        }
        Ok(Self { connection })
    }

    pub fn create(&self, new_pull_request: &NewPullRequest) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO pull_requests (title, description, source_branch, target_branch, author)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                new_pull_request.title,
                new_pull_request.description,
                new_pull_request.source_branch,
                new_pull_request.target_branch,
                new_pull_request.author
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Open pull requests first, then newest first.
    pub fn list(&self) -> Result<Vec<PullRequest>> {
        let mut statement = self
            .connection
            .prepare(&format!("SELECT {PULL_REQUEST_COLUMNS} FROM pull_requests ORDER BY status != 'open', id DESC"))?;
        let rows = statement.query_map([], read_pull_request_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get(&self, pull_request_id: i64) -> Result<PullRequest> {
        self.connection
            .query_row(
                &format!("SELECT {PULL_REQUEST_COLUMNS} FROM pull_requests WHERE id = ?1"),
                [pull_request_id],
                read_pull_request_row,
            )
            .optional()?
            .ok_or_else(|| Error(format!("pull request #{pull_request_id} does not exist")))
    }

    pub fn find_open_between(&self, source_branch: &str, target_branch: &str) -> Result<Option<i64>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id FROM pull_requests WHERE status = 'open' AND source_branch = ?1 AND target_branch = ?2",
                [source_branch, target_branch],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn update_title_and_description(&self, pull_request_id: i64, title: &str, description: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE pull_requests SET title = ?2, description = ?3 WHERE id = ?1",
            params![pull_request_id, title, description],
        )?;
        Ok(())
    }

    /// Only an open pull request can be finished, so a second merge or close never overwrites the first.
    pub fn finish(
        &self,
        pull_request_id: i64,
        final_status: PullRequestStatus,
        merge_method: Option<MergeMethod>,
        frozen_target_sha: &str,
        frozen_source_sha: &str,
    ) -> Result<()> {
        let changed_rows = self.connection.execute(
            "UPDATE pull_requests
             SET status = ?2, merged_or_closed_at = datetime('now'), frozen_target_sha = ?3, frozen_source_sha = ?4,
                 merge_method = ?5
             WHERE id = ?1 AND status = 'open'",
            params![
                pull_request_id,
                final_status.as_str(),
                frozen_target_sha,
                frozen_source_sha,
                merge_method.map(MergeMethod::as_str)
            ],
        )?;
        if changed_rows == 0 {
            return Err(Error(format!("pull request #{pull_request_id} is not open")));
        }
        Ok(())
    }
}

impl rusqlite::types::FromSql for PullRequestStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        PullRequestStatus::parse(value.as_str()?).map_err(|error| rusqlite::types::FromSqlError::Other(Box::new(error)))
    }
}

impl rusqlite::types::FromSql for MergeMethod {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        MergeMethod::parse(value.as_str()?).map_err(|error| rusqlite::types::FromSqlError::Other(Box::new(error)))
    }
}

fn read_pull_request_row(row: &Row) -> rusqlite::Result<PullRequest> {
    Ok(PullRequest {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        source_branch: row.get(3)?,
        target_branch: row.get(4)?,
        author: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        merged_or_closed_at: row.get(8)?,
        frozen_target_sha: row.get(9)?,
        frozen_source_sha: row.get(10)?,
        merge_method: row.get(11)?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedProject {
    pub repository_path: PathBuf,
    pub last_opened_at: String,
}

pub struct ProjectStore {
    connection: Connection,
}

impl ProjectStore {
    /// `~/Library/Application Support/Dialectic/projects.db` on macOS, `%APPDATA%\Dialectic` on Windows.
    pub fn open_in_app_data_dir() -> Result<Self> {
        let app_data_dir = std::env::var_os("DIALECTIC_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_dir().map(|data_dir| data_dir.join("Dialectic")))
            .ok_or_else(|| Error("could not find the user's application data directory".into()))?;
        std::fs::create_dir_all(&app_data_dir)?;
        Self::open(&app_data_dir.join("projects.db"))
    }

    pub fn open(database_path: &Path) -> Result<Self> {
        let connection = open_shared_connection(database_path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                repository_path TEXT PRIMARY KEY,
                last_opened_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
            );
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(Self { connection })
    }

    /// Adds the project, or moves it to the top if it is already saved.
    pub fn record_opened(&self, repository_path: &Path) -> Result<()> {
        self.connection.execute(
            "INSERT INTO projects (repository_path) VALUES (?1)
             ON CONFLICT (repository_path) DO UPDATE SET last_opened_at = strftime('%Y-%m-%d %H:%M:%f', 'now')",
            [repository_path.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn list_most_recent_first(&self) -> Result<Vec<SavedProject>> {
        let mut statement = self
            .connection
            .prepare("SELECT repository_path, last_opened_at FROM projects ORDER BY last_opened_at DESC")?;
        let rows = statement.query_map([], |row| {
            Ok(SavedProject { repository_path: PathBuf::from(row.get::<_, String>(0)?), last_opened_at: row.get(1)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Forgets the project; the repository and its pull requests are untouched.
    pub fn remove(&self, repository_path: &Path) -> Result<()> {
        self.connection
            .execute("DELETE FROM projects WHERE repository_path = ?1", [repository_path.to_string_lossy()])?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0))
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = ?2",
            [key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_pull_request<'a>(source_branch: &'a str) -> NewPullRequest<'a> {
        NewPullRequest {
            title: "Add login",
            description: "- [ ] tests",
            source_branch,
            target_branch: "main",
            author: "Ada",
        }
    }

    #[test]
    fn pull_requests_round_trip_and_finish_only_once() {
        let temporary_dir = tempfile::tempdir().unwrap();
        let store = PullRequestStore::open_in_git_dir(temporary_dir.path()).unwrap();
        let first_id = store.create(&new_pull_request("feature")).unwrap();
        let second_id = store.create(&new_pull_request("other")).unwrap();

        assert_eq!(store.find_open_between("feature", "main").unwrap(), Some(first_id));
        store.update_title_and_description(first_id, "Add login v2", "done").unwrap();
        store.finish(first_id, PullRequestStatus::Merged, Some(MergeMethod::Rebase), "aaa", "bbb").unwrap();
        assert!(store.finish(first_id, PullRequestStatus::Closed, None, "x", "y").is_err());

        let merged = store.get(first_id).unwrap();
        assert_eq!((merged.title.as_str(), merged.status), ("Add login v2", PullRequestStatus::Merged));
        assert_eq!(merged.frozen_source_sha.as_deref(), Some("bbb"));
        assert_eq!(merged.merge_method, Some(MergeMethod::Rebase));
        assert_eq!(store.find_open_between("feature", "main").unwrap(), None);

        let listed_ids: Vec<i64> = store.list().unwrap().iter().map(|pull_request| pull_request.id).collect();
        assert_eq!(listed_ids, vec![second_id, first_id], "open pull requests list first");
        assert!(store.get(99).is_err());
    }

    #[test]
    fn a_database_from_before_merge_methods_gains_the_column() {
        let temporary_dir = tempfile::tempdir().unwrap();
        let database_path = temporary_dir.path().join("dialectic.db");
        Connection::open(&database_path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE pull_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL,
                 description TEXT NOT NULL, source_branch TEXT NOT NULL, target_branch TEXT NOT NULL,
                 author TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'open',
                 created_at TEXT NOT NULL DEFAULT (datetime('now')), merged_or_closed_at TEXT,
                 frozen_target_sha TEXT, frozen_source_sha TEXT);
                 INSERT INTO pull_requests (title, description, source_branch, target_branch, author)
                 VALUES ('old', '', 'feature', 'main', 'Ada');",
            )
            .unwrap();
        let store = PullRequestStore::open(&database_path).unwrap();
        assert_eq!(store.get(1).unwrap().merge_method, None);
        store.finish(1, PullRequestStatus::Merged, Some(MergeMethod::MergeCommit), "a", "b").unwrap();
        assert_eq!(store.get(1).unwrap().merge_method, Some(MergeMethod::MergeCommit));
    }

    #[test]
    fn projects_are_deduplicated_and_settings_persist() {
        let temporary_dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::open(&temporary_dir.path().join("projects.db")).unwrap();
        store.record_opened(Path::new("/repos/a")).unwrap();
        store.record_opened(Path::new("/repos/b")).unwrap();
        store.record_opened(Path::new("/repos/a")).unwrap();
        let saved_paths: Vec<PathBuf> =
            store.list_most_recent_first().unwrap().into_iter().map(|project| project.repository_path).collect();
        assert_eq!(saved_paths, vec![PathBuf::from("/repos/a"), PathBuf::from("/repos/b")]);
        store.remove(Path::new("/repos/b")).unwrap();
        assert_eq!(store.list_most_recent_first().unwrap().len(), 1);

        assert_eq!(store.setting("diff_mode").unwrap(), None);
        store.set_setting("diff_mode", "split").unwrap();
        assert_eq!(store.setting("diff_mode").unwrap().as_deref(), Some("split"));
    }
}
