//! Detection and reading of the five sources against real directories and stub tools.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use repoview_sources::{
    Availability, DIAGNOSTIC_LIMIT, Env, Outcome, Section, SourceKind, read_all, run,
    truncate_diagnostic,
};
use serde_json::json;
use tempfile::TempDir;

/// The real binary called `name` on the test process's `PATH`.
fn real_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{name} is on the test PATH"))
}

/// Write an executable `/bin/sh` stub and wait until it can be executed (no ETXTBSY from a
/// concurrently forked test thread still holding the write descriptor).
fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("--version").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return path,
        }
    }
    panic!("stub {name} stayed busy");
}

/// A directory to use as `PATH`, holding symlinks to the named real tools.
fn bin_dir(real: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in real {
        std::os::unix::fs::symlink(real_tool(name), dir.path().join(name)).unwrap();
    }
    dir
}

fn git(dir: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git_repo_with_commit() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    git(
        dir.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "first"],
    );
    dir
}

fn section<'a>(sections: &'a [Section], id: &str) -> &'a Section {
    sections
        .iter()
        .find(|section| section.source_id == id)
        .unwrap_or_else(|| panic!("no section {id}"))
}

fn project_with_plan() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".engineering")).unwrap();
    fs::write(dir.path().join(".engineering/project.yaml"), "name: x\n").unwrap();
    dir
}

#[test]
fn five_sources_in_wire_order_with_their_kinds() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let ids: Vec<_> = sections
        .iter()
        .map(|s| (s.source_id.as_str(), s.kind))
        .collect();
    assert_eq!(
        ids,
        vec![
            ("vcs", SourceKind::Vcs),
            ("plan", SourceKind::Planning),
            ("spec", SourceKind::Specification),
            ("quality", SourceKind::Quality),
            ("docs", SourceKind::Documents),
        ]
    );
}

#[test]
fn section_serialises_to_the_wire_shape() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let value = serde_json::to_value(section(&sections, "docs")).unwrap();
    assert_eq!(
        value,
        json!({
            "source_id": "docs",
            "kind": "Documents",
            "location": ".",
            "availability": "Absent",
            "tool": null,
            "tool_path": null,
            "tool_version": null,
            "diagnostic": null,
            "summary": {}
        })
    );
}

#[test]
fn vcs_present_reports_branch_head_and_dirty_count() {
    let project = git_repo_with_commit();
    let path = bin_dir(&["git"]);
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let vcs = section(&sections, "vcs");
    assert_eq!(vcs.availability, Availability::Present);
    assert_eq!(vcs.tool.as_deref(), Some("git"));
    assert!(vcs.tool_path.is_some());
    assert!(
        vcs.tool_version
            .as_deref()
            .unwrap()
            .starts_with("git version")
    );
    assert_eq!(
        vcs.summary["branch"],
        json!(git(project.path(), &["branch", "--show-current"]))
    );
    assert_eq!(
        vcs.summary["head"],
        json!(git(project.path(), &["rev-parse", "HEAD"]))
    );
    assert_eq!(vcs.summary["dirty"], json!(0));
}

#[test]
fn vcs_tool_missing_without_git_on_path() {
    let project = git_repo_with_commit();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let vcs = section(&sections, "vcs");
    assert_eq!(vcs.availability, Availability::ToolMissing);
    assert_eq!(vcs.tool.as_deref(), Some("git"));
}

#[test]
fn plan_absent_without_project_yaml() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    assert_eq!(
        section(&sections, "plan").availability,
        Availability::Absent
    );
}

#[test]
fn plan_tool_missing_when_aep_is_not_on_path() {
    let project = project_with_plan();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let plan = section(&sections, "plan");
    assert_eq!(plan.availability, Availability::ToolMissing);
    assert_eq!(plan.tool.as_deref(), Some("aep"));
    assert_eq!(plan.tool_path, None);
}

#[test]
fn plan_present_with_the_stub_version() {
    let project = project_with_plan();
    let path = tempfile::tempdir().unwrap();
    let aep = stub(path.path(), "aep", "printf 'aep 9.9.9\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let plan = section(&sections, "plan");
    assert_eq!(plan.availability, Availability::Present);
    assert_eq!(plan.tool.as_deref(), Some("aep"));
    assert_eq!(plan.tool_version.as_deref(), Some("aep 9.9.9"));
    assert_eq!(plan.tool_path.as_deref(), Some(aep.to_str().unwrap()));
    assert_eq!(plan.diagnostic, None);
}

#[test]
fn plan_failed_carries_the_tool_stderr() {
    let project = project_with_plan();
    let path = tempfile::tempdir().unwrap();
    stub(path.path(), "aep", "echo boom >&2\nexit 3");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let plan = section(&sections, "plan");
    assert_eq!(plan.availability, Availability::Failed);
    assert!(
        plan.diagnostic.as_deref().unwrap().contains("boom"),
        "{plan:?}"
    );
}

#[test]
fn failed_diagnostic_is_truncated_to_four_kib() {
    let project = project_with_plan();
    let path = tempfile::tempdir().unwrap();
    stub(
        path.path(),
        "aep",
        "i=0\nwhile [ $i -lt 1000 ]; do printf 'boom-boom ' >&2; i=$((i+1)); done\nexit 1",
    );
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let plan = section(&sections, "plan");
    assert_eq!(plan.availability, Availability::Failed);
    let diagnostic = plan.diagnostic.as_deref().unwrap();
    assert!(diagnostic.len() <= DIAGNOSTIC_LIMIT, "{}", diagnostic.len());
    assert!(diagnostic.starts_with("boom-boom"));
}

#[test]
fn truncation_keeps_utf8_boundaries() {
    let text = "ä".repeat(DIAGNOSTIC_LIMIT);
    let cut = truncate_diagnostic(&text);
    assert!(cut.len() <= DIAGNOSTIC_LIMIT);
    assert!(cut.len() >= DIAGNOSTIC_LIMIT - 1);
    assert_eq!(truncate_diagnostic("short"), "short");
}

#[test]
fn run_kills_a_tool_past_its_timeout() {
    let project = tempfile::tempdir().unwrap();
    let env = Env::new(project.path());
    let started = Instant::now();
    let outcome = run(
        &env,
        &real_tool("sleep"),
        &["30"],
        Duration::from_millis(200),
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    match outcome {
        Outcome::Failure { diagnostic } => {
            assert!(diagnostic.contains("timed out"), "{diagnostic}")
        }
        other => panic!("expected a timeout failure, got {other:?}"),
    }
}

#[test]
fn run_uses_the_project_root_as_working_directory() {
    let project = tempfile::tempdir().unwrap();
    let env = Env::new(project.path());
    match run(&env, &real_tool("pwd"), &[], Duration::from_secs(10)) {
        Outcome::Success { stdout } => assert_eq!(
            fs::canonicalize(stdout.trim()).unwrap(),
            fs::canonicalize(project.path()).unwrap()
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn spec_absent_without_specification_files() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    assert_eq!(
        section(&sections, "spec").availability,
        Availability::Absent
    );
}

#[test]
fn spec_roots_skip_git_ignored_paths() {
    let project = git_repo_with_commit();
    fs::write(project.path().join(".gitignore"), "ignored/\n").unwrap();
    for dir in ["ess", "nested/model", "ignored"] {
        fs::create_dir_all(project.path().join(dir)).unwrap();
    }
    fs::write(project.path().join("ess/ess-inputs.yaml"), "x").unwrap();
    fs::write(project.path().join("ess/system.yaml"), "x").unwrap();
    fs::write(project.path().join("nested/model/system.yaml"), "x").unwrap();
    fs::write(project.path().join("ignored/system.yaml"), "x").unwrap();
    let path = bin_dir(&["git"]);
    stub(path.path(), "ess", "printf 'ess 7.7.7\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let spec = section(&sections, "spec");
    assert_eq!(spec.availability, Availability::Present, "{spec:?}");
    assert_eq!(spec.tool.as_deref(), Some("ess"));
    assert_eq!(spec.tool_version.as_deref(), Some("ess 7.7.7"));
    assert_eq!(spec.summary, json!({ "roots": ["ess", "nested/model"] }));
}

#[test]
fn spec_outside_git_walks_the_tree() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("system.yaml"), "x").unwrap();
    let path = tempfile::tempdir().unwrap();
    stub(path.path(), "ess", "printf 'ess 7.7.7\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let spec = section(&sections, "spec");
    assert_eq!(spec.availability, Availability::Present, "{spec:?}");
    assert_eq!(spec.summary, json!({ "roots": ["."] }));
}

#[test]
fn spec_tool_missing_when_ess_is_not_on_path() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("ess-inputs.yaml"), "x").unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let spec = section(&sections, "spec");
    assert_eq!(spec.availability, Availability::ToolMissing);
    assert_eq!(spec.tool.as_deref(), Some("ess"));
}

#[test]
fn quality_absent_without_codegate_not_tool_missing() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let quality = section(&sections, "quality");
    assert_eq!(quality.availability, Availability::Absent);
    assert_eq!(quality.tool.as_deref(), Some("codegate"));
}

#[test]
fn quality_present_with_codegate_on_path() {
    let project = tempfile::tempdir().unwrap();
    let path = tempfile::tempdir().unwrap();
    // story:quality-codegate: only a codegate whose --version prints `codegate <semver>` counts.
    let codegate = stub(path.path(), "codegate", "printf 'codegate 0.3.0\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let quality = section(&sections, "quality");
    assert_eq!(quality.availability, Availability::Present);
    assert_eq!(
        quality.tool_path.as_deref(),
        Some(codegate.to_str().unwrap())
    );
    assert_eq!(quality.tool_version.as_deref(), Some("0.3.0"));
}

#[test]
fn docs_present_lists_the_documents_found() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("README.md"), "# x").unwrap();
    fs::write(project.path().join("CHANGELOG.md"), "# x").unwrap();
    fs::write(project.path().join("NOTES.md"), "# x").unwrap();
    let path = tempfile::tempdir().unwrap();
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let docs = section(&sections, "docs");
    assert_eq!(docs.availability, Availability::Present);
    assert_eq!(docs.tool, None);
    assert_eq!(
        docs.summary,
        json!({ "files": ["README.md", "CHANGELOG.md"] })
    );
}

// Correction round 1.

/// Finding 1: every child starts with `GIT_OPTIONAL_LOCKS=0`, so no git invocation refreshes and
/// rewrites the index inside the project.
#[test]
fn every_child_runs_without_optional_git_locks() {
    let project = tempfile::tempdir().unwrap();
    let env = Env::new(project.path());
    match run(&env, &real_tool("env"), &[], Duration::from_secs(10)) {
        Outcome::Success { stdout } => assert!(
            stdout.lines().any(|line| line == "GIT_OPTIONAL_LOCKS=0"),
            "{stdout}"
        ),
        other => panic!("{other:?}"),
    }
}

/// Finding 8: outside Git the walk stops below depth 6.
#[test]
fn spec_walk_outside_git_stops_below_depth_six() {
    let project = tempfile::tempdir().unwrap();
    let six = project.path().join("a/b/c/d/e/f");
    let seven = six.join("g");
    fs::create_dir_all(&seven).unwrap();
    fs::write(six.join("system.yaml"), "x").unwrap();
    fs::write(seven.join("system.yaml"), "x").unwrap();
    let path = tempfile::tempdir().unwrap();
    stub(path.path(), "ess", "printf 'ess 7.7.7\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    let spec = section(&sections, "spec");
    assert_eq!(
        spec.summary,
        json!({ "roots": ["a/b/c/d/e/f"], "truncated": true }),
        "{spec:?}"
    );
}

/// Finding 8: outside Git the walk skips `target`, `node_modules` and `.git`.
#[test]
fn spec_walk_outside_git_skips_build_and_vendor_directories() {
    let project = tempfile::tempdir().unwrap();
    for dir in ["target/x", "node_modules/y", ".git/z", "model"] {
        fs::create_dir_all(project.path().join(dir)).unwrap();
        fs::write(project.path().join(dir).join("system.yaml"), "x").unwrap();
    }
    let path = tempfile::tempdir().unwrap();
    stub(path.path(), "ess", "printf 'ess 7.7.7\\n'");
    let sections = read_all(&Env::with_path(project.path(), path.path()));
    assert_eq!(
        section(&sections, "spec").summary,
        json!({ "roots": ["model"] })
    );
}

// Correction round 2.

/// Finding 2: every child also runs with `core.fsmonitor` and `core.untrackedCache` off through
/// `GIT_CONFIG_COUNT`, so no git invocation starts a daemon or writes a cache into `.git`.
#[test]
fn every_child_runs_with_fsmonitor_and_untracked_cache_off() {
    let project = tempfile::tempdir().unwrap();
    let env = Env::new(project.path());
    let Outcome::Success { stdout } = run(&env, &real_tool("env"), &[], Duration::from_secs(10))
    else {
        panic!("env failed");
    };
    let lines: Vec<&str> = stdout.lines().collect();
    for expected in [
        "GIT_CONFIG_COUNT=2",
        "GIT_CONFIG_KEY_0=core.fsmonitor",
        "GIT_CONFIG_VALUE_0=false",
        "GIT_CONFIG_KEY_1=core.untrackedCache",
        "GIT_CONFIG_VALUE_1=false",
    ] {
        assert!(lines.contains(&expected), "{expected} missing:\n{stdout}");
    }
}

/// Finding 2, end to end: a repository configured with `core.fsmonitor=true` and
/// `core.untrackedCache=true` keeps `.git` unchanged across a snapshot.
#[test]
fn snapshot_leaves_dot_git_unchanged_under_fsmonitor_and_untracked_cache() {
    let project = git_repo_with_commit();
    git(project.path(), &["config", "core.fsmonitor", "true"]);
    git(project.path(), &["config", "core.untrackedCache", "true"]);
    fs::write(project.path().join("untracked.txt"), "x").unwrap();
    let listing = |dir: &Path| -> Vec<(String, u64)> {
        let mut entries: Vec<_> = fs::read_dir(dir.join(".git"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    entry.metadata().unwrap().len(),
                )
            })
            .collect();
        entries.sort();
        entries
    };
    let before = listing(project.path());
    let index = fs::read(project.path().join(".git/index")).unwrap();
    let path = bin_dir(&["git"]);
    let _ = read_all(&Env::with_path(project.path(), path.path()));
    let _ = Command::new("git")
        .args(["fsmonitor--daemon", "stop"])
        .current_dir(project.path())
        .output();
    assert_eq!(listing(project.path()), before);
    assert_eq!(fs::read(project.path().join(".git/index")).unwrap(), index);
}
