# Changelog

All notable changes to Dialectic are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- The Windows app no longer opens a console window. The release also ships `dialectic.com`, which
  terminals pick over `dialectic.exe`, so `dialectic pr …` still prints and waits in the terminal.

## [0.1.0] - 2026-11-18

First release, built for Windows.

### Added

- Pull requests for a local Git repository, stored in SQLite: open, list, edit, close and merge.
- Merging with a merge commit or by rebasing, with a merge method picker.
- Merge conflicts predicted without touching a checkout; merging is blocked while a pull
  request's branches conflict.
- Welcome screen with saved projects and a native folder picker.
- New pull request form with a Markdown editor and Write/Preview tabs.
- Pull request detail with Overview, Commits and Files changed tabs; the diff has line numbers,
  word-level highlights and a Unified/Split toggle.
- Your changes screen: see uncommitted changes, commit them, switch branches or start a branch
  that takes them along; open pull requests are called out.
- `dialectic pr` CLI commands with JSON output and a guide for agents.
- Spanish translation, with the language picker on a Settings screen that also shows the
  version and a link to the source code.
- Icons on buttons and file-type icons in the changed-file list.
- A changed-file list that stays fast with thousands of files, and uncommitted changes that load
  in one `git diff` however many files are new.

[Unreleased]: https://github.com/namzug16/dialectic/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/namzug16/dialectic/releases/tag/v0.1.0
