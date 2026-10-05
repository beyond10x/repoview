//! Adversary cases, pass 2, for story:server-skeleton: the bounded walk outside Git and the
//! read-only rule under a user's Git configuration.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use repoview_sources::{Availability, Env, Section, read_all};

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

fn section<'a>(sections: &'a [Section], id: &str) -> &'a Section {
    sections
        .iter()
        .find(|section| section.source_id == id)
        .unwrap_or_else(|| panic!("no section {id}"))
}

/// Detection: `spec` = any `system.yaml` under the root. Outside Git, a marker one level below the
/// root must not be missed because a sibling directory the walk happened to enter first holds
/// more than the 20,000-entry budget (a Python `.venv`, a `vendor/`, an unpacked `dist/`).
///
/// The walk is depth-first in `read_dir` order, so the test reads that order and fills whichever
/// directory comes first; the result does not depend on the file system's hash order.
#[test]
fn spec_outside_git_finds_a_shallow_marker_beside_a_large_directory() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("bulk-a")).unwrap();
    fs::create_dir(project.path().join("bulk-b")).unwrap();
    let order: Vec<_> = fs::read_dir(project.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    let (first, second) = (
        project.path().join(&order[0]),
        project.path().join(&order[1]),
    );
    for index in 0..20_001 {
        fs::write(first.join(format!("f{index}")), "").unwrap();
    }
    fs::write(second.join("system.yaml"), "x").unwrap();

    // No `git` on PATH: the walk outside Git runs. No `ess` either: detection alone is asserted.
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let spec = section(&sections, "spec");
    assert_ne!(
        spec.availability,
        Availability::Absent,
        "{} missed after {} was walked first: {spec:?}",
        second.join("system.yaml").display(),
        first.display()
    );
}

/// AGENTS.md: "Read-only. repoview writes nothing inside the project directory." With the
/// user's `core.fsmonitor=true` (Scalar sets it; users set it globally for speed), the `git
/// status` a snapshot runs starts `git fsmonitor--daemon`, which creates
/// `.git/fsmonitor--daemon.ipc` and `.git/fsmonitor--daemon/` inside the project and keeps
/// running after repoview's child has exited.
#[test]
fn snapshot_does_not_start_a_fsmonitor_daemon_in_the_project() {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "--quiet"]);
    git(
        repo.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "first"],
    );
    git(repo.path(), &["config", "core.fsmonitor", "true"]);
    let ipc = repo.path().join(".git/fsmonitor--daemon.ipc");
    let cookies = repo.path().join(".git/fsmonitor--daemon");
    assert!(!ipc.exists() && !cookies.exists());

    let bin = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(real_tool("git"), bin.path().join("git")).unwrap();
    let _ = read_all(&Env::with_path(repo.path(), bin.path()));

    let started = ipc.exists() || cookies.exists();
    // Stop any daemon before asserting, so a red run leaves no process behind.
    let _ = Command::new("git")
        .args(["fsmonitor--daemon", "stop"])
        .current_dir(repo.path())
        .output();
    assert!(
        !started,
        "the snapshot started git fsmonitor--daemon, which wrote into {}",
        repo.path().join(".git").display()
    );
}
