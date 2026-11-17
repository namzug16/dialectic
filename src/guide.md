# Dialectic: opening pull requests as an agent

Dialectic keeps GitHub-style pull requests for a **local** Git repository. Developers (people
and agents) work on their own branches of the same repository on one machine. A pull request
proposes merging one branch into another. A human reviews it in the Dialectic desktop app and
merges it there. **You cannot merge.** Your job is to make the change easy to review.

## Workflow

1. Work on your own branch, never directly on the target branch (`git switch -c my-change`).
2. Commit your work. Only committed changes are part of a pull request.
3. Open the pull request from your branch:

   ```sh
   dialectic pr create --target main --title "Add punctuation to greetings" --description-file - <<'EOF'
   ## What changed
   ...
   EOF
   ```

   `--source` defaults to your current branch. Use `-C <path>` to point at a repository other
   than the current directory.
4. Check it: `dialectic pr show <id>` lists the commits and changed files the reviewer will see;
   `dialectic pr diff <id>` prints the exact diff.
5. Need more changes? Commit them to the same branch: the open pull request picks them up
   automatically. Then update the description if it no longer matches:
   `dialectic pr edit <id> --description-file -`.
6. Abandoning the change? `dialectic pr close <id>`.

## Rules (the command fails with a message if you break one)

- Source and target must be two different local branches.
- Only one open pull request per source → target pair. Edit the existing one instead
  (`dialectic pr list --status open` finds it).
- The source must have commits the target doesn't have.
- The title can't be empty. Merged and closed pull requests can't be edited.

## Writing the description

The description is Markdown (headings, lists, `- [ ]` task lists, links, `inline code`, fenced
code blocks). A reviewer should understand the change without reading every line:

```markdown
## What changed
One or two sentences on the behaviour change, and why.

## How to check it
- [x] Tests pass: `pytest`
- [ ] Try `greet('Ada')` and expect `Hello, Ada!`

## Notes for the reviewer
Anything surprising, risky or left for later.
```

Keep the title short and imperative ("Add punctuation to greetings").

## Commands

| Command | What it does |
|---|---|
| `dialectic pr create --target T --title X [--source S] [--description D \| --description-file F\|-]` | Open a pull request |
| `dialectic pr list [--status open\|merged\|closed]` | List pull requests, open first |
| `dialectic pr show <id>` | Title, status, description, commits, changed files |
| `dialectic pr diff <id>` | The unified diff |
| `dialectic pr edit <id> [--title X] [--description D \| --description-file F\|-]` | Update an open pull request |
| `dialectic pr close <id>` | Close without merging |

Add `--json` to `create`, `list`, `show` and `edit` for machine-readable output (one JSON value
on stdout). Exit codes: `0` success, `1` the command failed (reason on stderr), `2` invalid
arguments. `dialectic <command> --help` shows every flag.
