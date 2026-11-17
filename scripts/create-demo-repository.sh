#!/usr/bin/env bash
# Creates a disposable repository for trying Dialectic, as two developers on one machine would use it:
#   <dir>          Grace's checkout, on main
#   <dir>-linus    Linus's worktree of the same repository, on feature
# Branches: feature merges cleanly into main; conflict changes the line main changed.
# With a dialectic binary as the second argument, Linus also opens two pull requests with its CLI.
set -euo pipefail

repository_dir="${1:?usage: create-demo-repository.sh <directory> [dialectic-binary]}"
dialectic_binary="${2:-}"
second_worktree_dir="$repository_dir-linus"

if [ -d "$repository_dir/.git" ]; then git -C "$repository_dir" worktree remove --force "$second_worktree_dir" 2>/dev/null || true; fi
rm -rf "$repository_dir" "$second_worktree_dir"
mkdir -p "$repository_dir"
cd "$repository_dir"

git init -q -b main
git config commit.gpgsign false
git config user.email developers@example.com
# Each worktree keeps its own user.name, so pull requests and commits say who made them.
git config extensions.worktreeConfig true
git config --worktree user.name "Grace"

commit_as() {
    local author="$1" file="$2" contents="$3" message="$4"
    printf '%b' "$contents" > "$file"
    git add "$file"
    git -c user.name="$author" commit -q -m "$message"
}

commit_as Grace README.md "# Greeter\n\nPrints a greeting.\n" "docs: add readme"
commit_as Grace greeter.py "def greet(name):\n    return 'Hello, ' + name\n\n\nif __name__ == '__main__':\n    print(greet('world'))\n" "feat: greet the world"

git switch -q -c feature
commit_as Linus greeter.py "def greet(name, punctuation='!'):\n    return f'Hello, {name}{punctuation}'\n\n\nif __name__ == '__main__':\n    print(greet('world'))\n" "feat: add punctuation to greetings"
commit_as Linus test_greeter.py "from greeter import greet\n\n\ndef test_greet():\n    assert greet('Ada') == 'Hello, Ada!'\n" "test: cover greet"

git switch -q -c conflict main
commit_as Linus README.md "# Greeter\n\nPrints a friendly greeting.\n" "docs: friendlier readme"

git switch -q main
commit_as Grace README.md "# Greeter\n\nPrints a greeting to the terminal.\n" "docs: say where the greeting goes"

git worktree add -q "$second_worktree_dir" feature
git -C "$second_worktree_dir" config --worktree user.name "Linus"

if [ -n "$dialectic_binary" ]; then
    "$dialectic_binary" -C "$second_worktree_dir" pr create --target main --title "Add punctuation to greetings" --description-file - <<'EOF'
## What changed

Greetings now end with punctuation, `!` by default.

- [x] Add a `punctuation` parameter
- [x] Cover `greet` with a test
- [ ] Document it in the [README](https://example.com/readme)

```python
greet('Ada', punctuation='?')  # Hello, Ada?
```
EOF
    "$dialectic_binary" -C "$second_worktree_dir" pr create --source conflict --target main --title "Friendlier readme" \
        --description "Rewords the README. Grace changed the same line on main, so this one conflicts."
fi

echo "Demo ready: $repository_dir (Grace, on main) and $second_worktree_dir (Linus, on feature)"
