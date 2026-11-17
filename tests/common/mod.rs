//! A disposable repository: `main`, `feature` (merges cleanly into main) and `conflict` (doesn't).

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub fn git(repository_dir: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git").arg("-C").arg(repository_dir).args(arguments).output().unwrap();
    assert!(output.status.success(), "git {arguments:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit_file(repository_dir: &Path, file_name: &str, contents: &str, message: &str) {
    std::fs::write(repository_dir.join(file_name), contents).unwrap();
    git(repository_dir, &["add", file_name]);
    git(repository_dir, &["commit", "-q", "-m", message]);
}

pub fn create_demo_repository(repository_dir: &Path) {
    git(repository_dir, &["init", "-q", "-b", "main"]);
    for (key, value) in [("user.name", "Grace"), ("user.email", "grace@example.com"), ("commit.gpgsign", "false")] {
        git(repository_dir, &["config", key, value]);
    }
    commit_file(repository_dir, "app.txt", "one\ntwo\nthree\nfour\nfive\nsix\n", "initial commit");

    git(repository_dir, &["switch", "-q", "-c", "feature"]);
    commit_file(repository_dir, "greeting.txt", "hello\n", "add greeting");
    commit_file(repository_dir, "app.txt", "one\ntwo\nthree\nfour\nfive\nsix (feature)\n", "tweak six");

    git(repository_dir, &["switch", "-q", "-c", "conflict", "main"]);
    commit_file(repository_dir, "app.txt", "one (conflict)\ntwo\nthree\nfour\nfive\nsix\n", "change one on conflict");

    git(repository_dir, &["switch", "-q", "main"]);
    commit_file(repository_dir, "app.txt", "one (main)\ntwo\nthree\nfour\nfive\nsix\n", "change one on main");
}
