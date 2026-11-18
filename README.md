# Dialectic

GitHub-style pull requests for a **local** Git repository. Two developers (or a developer and
their agents) work on different branches of the same repository on one machine. One opens a
pull request from their branch; the other reviews its commits and diff in Dialectic and merges
it with a standard `git merge`.

Built in Rust on [Rune](https://github.com/anomalyco/rune) (Anomaly's GPU-drawn UI framework),
pinned to `dev` revision `842c28c2ca5dd1506c1e4a67052db61800aed721`.

## Quick start

```sh
make demo                 # disposable two-developer repo with two pull requests, opened in the app
make run REPO=~/code/app  # open your own repository (or run `make run` and pick one)
cargo run --release -- /path/to/repo   # the same without make
```

`make help` lists every target: `run`, `build`, `test`, `check` (fmt, clippy, tests), `demo`,
`screenshots`, `install`, `clean`.

## The app

- **Welcome** (Xcode-style): Open Repository… picks a folder with the native dialog; opened
  repositories are remembered (`~/Library/Application Support/Dialectic/projects.db`).
- **Pull requests**: open, merged and closed pull requests with their `source → target`
  branches, and New pull request. The list reloads each time you return to it, so pull
  requests opened by someone else's app or the CLI appear.
- **New pull request**: pick two local branches, write a title and a Markdown description with
  Write and Preview tabs (headings, lists, task checkboxes, links, inline code, fenced code).
- **Pull request detail**: title, status, branches and rendered description, with Overview,
  Commits and Files changed tabs. Files changed is a file list beside a diff with line numbers,
  word-level highlights and a remembered Unified/Split toggle (`j`/`k` hunks, `]`/`[` files).
  Overview has Edit, Close pull request, and the merge button with its Merge method picker.
- **Your changes** (the first screen of a repository): the opened checkout's uncommitted
  changes, untracked files included, in the same file list and diff. With nothing to commit it
  says so in the middle of the window and, while any pull requests are open, shows how many are
  waiting with a green button to them (the header's Pull requests button turns green too). Commit them all with one message (no staging area),
  switch branch (refused while there are uncommitted changes), or start a new branch that takes
  the changes along. On `main`/`master` it warns you to branch first; once everything is
  committed on a work branch it offers Open a pull request.
- **English or Español**: a picker on the welcome screen and in the header switches every label
  and remembers the choice. All labels live in `src/ui/strings.rs`; errors from git and the
  database stay in English, as the CLI shares them.

### How merging works

Pick a merge method above the merge button, as on GitHub:

- **Create a merge commit** runs `git merge --no-ff <source>` into the target branch.
- **Rebase and merge** replays the source's commits onto the target in a temporary detached
  worktree, then fast-forwards the target to them: a linear history, no merge commit. The source
  branch itself never moves.

The target is updated in the worktree that has it checked out (Dialectic refuses if that
checkout has uncommitted changes), or, when no worktree has it, without touching any checkout.

Conflicts are found before anyone merges: opening a pull request runs `git merge-tree`
(git 2.38+, touches no checkout). While the branches conflict, the merge buttons are hidden, a
red banner shows on every tab, the conflicting files come first in Files changed (in red, with a
Conflict tag), and the merge box tells the reviewer to ask the source branch's owner to fix them,
or gives a prompt for a coding agent. Once the fix is committed, open the pull request again.

If git succeeds, the pull request is marked merged (and remembers how). If a merge still
conflicts (a rebase can conflict where `merge-tree` predicted a clean merge), Dialectic aborts
it, leaves the target exactly as it was and blocks the merge the same way.

Commits and the diff are read live from the branches while a pull request is open (new commits
on the source show up automatically), and frozen when it is merged or closed.

### Where data lives

Pull requests are stored in `dialectic.db` (SQLite) inside the repository's shared Git
directory (`git rev-parse --git-common-dir`), so every worktree, app window and CLI call sees
the same pull requests.

## The CLI, for agents

`make install` puts `dialectic` on your PATH. Agents open and maintain pull requests from the
terminal; merging stays in the app, for a human.

```sh
dialectic guide                                    # the workflow and rules, written for agents
dialectic pr create --target main --title "…" --description-file -   # source: current branch
dialectic pr list [--status open|merged|closed] [--json]
dialectic pr show <id> [--json]                    # commits and changed files
dialectic pr diff <id>
dialectic pr edit <id> [--title …] [--description … | --description-file …]
dialectic pr close <id>
```

`-C <path>` picks the repository (any worktree). Exit codes: 0 success, 1 failure (reason on
stderr), 2 invalid arguments. To teach your agents, add this to your `AGENTS.md` or `CLAUDE.md`:

> To open a pull request, run `dialectic guide` and follow it.

## Development

```sh
make check        # cargo fmt --check, clippy -D warnings, cargo test
make screenshots  # renders every screen headless into target/screenshots
```

Tests cover the core against disposable repositories (`tests/repository.rs`), the CLI binary
(`tests/cli.rs`) and the GUI clicked through headless with Rune's `Runtime` (`tests/ui.rs`).
Screens can be rendered on their own: `target/debug/dialectic <repo> --scene <name> --snap out.png`
(scenes are listed in `src/ui/mod.rs`; Rune's headless flags exist in debug builds).

Cargo fetches Rune's git dependencies with the git CLI (`.cargo/config.toml`), because one of
Rune's pinned revisions is unreachable with cargo's built-in fetcher. To update Rune, change the
`rev` everywhere in `Cargo.toml`, including `[patch.crates-io]`, and copy that table from Rune's
root `Cargo.toml` at the new revision.

## Windows (not yet built)

The code avoids macOS-only paths, but Windows builds haven't been tried. What a Windows build needs:

- the MSVC toolchain (Rune's README lists MSVC or MinGW binutils) and Rust 1.99.1
  (`rust-toolchain.toml` installs it);
- `git` on `PATH`, since Dialectic runs the git CLI;
- `CARGO_NET_GIT_FETCH_WITH_CLI=true`, already set by `.cargo/config.toml`;
- plain `cargo` commands instead of `make` (the Makefile and the demo script assume a Unix
  shell): `cargo run --release -- C:\path\to\repo`, `cargo test`.

Saved projects go to `%APPDATA%\Dialectic\projects.db`. Rune's native folder dialog uses rfd on
Windows.
