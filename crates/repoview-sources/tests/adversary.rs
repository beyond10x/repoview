//! Adversary cases for story:server-skeleton: subprocess bounds and the read-only rule.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

use repoview_sources::{Env, read_all, run};

fn real_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{name} is on the test PATH"))
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=repoview test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// AGENTS.md: "Read-only. repoview writes nothing inside the project directory." A snapshot of a
/// repository whose index has stale stat data must leave `.git/index` byte-for-byte unchanged
/// (`git status` refreshes and rewrites the index unless run with `--no-optional-locks`).
#[test]
fn snapshot_does_not_rewrite_the_git_index() {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "--quiet"]);
    fs::write(repo.path().join("a.txt"), "same content\n").unwrap();
    git(repo.path(), &["add", "a.txt"]);
    git(repo.path(), &["commit", "--quiet", "-m", "first"]);
    // Same content, older mtime: the index entry's stat data is now stale.
    fs::File::options()
        .write(true)
        .open(repo.path().join("a.txt"))
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(946_684_800))
        .unwrap();
    let index = repo.path().join(".git/index");
    let before = fs::read(&index).unwrap();

    let bin = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(real_tool("git"), bin.path().join("git")).unwrap();
    let _ = read_all(&Env::with_path(repo.path(), bin.path()));

    assert!(
        fs::read(&index).unwrap() == before,
        "the snapshot rewrote .git/index inside the project directory"
    );
}

/// The design gives each subprocess a 10 s timeout. A tool that exits at once but leaves a
/// background child holding its stdout must still return within the timeout.
#[test]
fn run_returns_within_the_timeout_when_a_grandchild_keeps_stdout_open() {
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("tool");
    fs::write(
        &tool,
        "#!/bin/sh\nif [ \"$1\" = go ]; then sleep 6 & fi\necho v\n",
    )
    .unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&tool).output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => break,
        }
    }
    let env = Env::new(dir.path());
    let started = Instant::now();
    let outcome = run(&env, &tool, &["go"], Duration::from_secs(1));
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "run with a 1 s timeout took {elapsed:?} ({outcome:?})"
    );
}
