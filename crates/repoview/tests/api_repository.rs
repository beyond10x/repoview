//! story:repository-page acceptance 1 and 3: `/api/vcs`, `/api/docs` and `/api/docs/{name}` against
//! real Git repositories built in temporary directories.

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tempfile::TempDir;

use common::{bin_dir, canonical, real_tool};

const PORT: u16 = 7480;
const HOST: &str = "127.0.0.1:7480";

fn token() -> String {
    "a".repeat(64)
}

fn state() -> AppState {
    AppState {
        token: token(),
        port: PORT,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    }
}

/// `GET uri` with the token, against the project at `root` with `PATH` = `path`.
async fn get_raw(root: &Path, path: &Path, uri: &str) -> (StatusCode, Vec<u8>) {
    let response = tower::ServiceExt::oneshot(
        router(state()).layer(Extension(Env::with_path(root, path))),
        Request::get(uri)
            .header(header::HOST, HOST)
            .header(TOKEN_HEADER, token())
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, body.to_vec())
}

async fn get(root: &Path, path: &Path, uri: &str) -> (StatusCode, Value) {
    let (status, body) = get_raw(root, path, uri).await;
    let value = serde_json::from_slice(&body).unwrap_or_else(|error| {
        panic!(
            "{uri} answered {status} with non-JSON ({error}): {}",
            String::from_utf8_lossy(&body)
        )
    });
    (status, value)
}

/// `/api/vcs` with the real `git`, asserting 200.
async fn vcs(root: &Path) -> Value {
    let path = bin_dir(&["git"]);
    let (status, body) = get(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// `git args…` in `dir` with a fixed identity, no global or system config, and `env` added.
fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=repoview test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git(dir: &Path, args: &[&str]) -> String {
    git_env(dir, args, &[])
}

fn commit(dir: &Path, subject: &str) -> String {
    git(dir, &["commit", "--quiet", "--allow-empty", "-m", subject]);
    git(dir, &["rev-parse", "HEAD"])
}

/// A repository on `main` with no commits.
fn unborn() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet", "-b", "main"]);
    dir
}

fn repo_with_commit() -> TempDir {
    let dir = unborn();
    commit(dir.path(), "first");
    dir
}

fn strings(values: &Value, field: &str) -> Vec<String> {
    values
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value[field].as_str().unwrap().to_owned())
        .collect()
}

// ---- /api/vcs ------------------------------------------------------------------------------

/// Local `main` two commits ahead of and one behind `origin/main`, against a real bare remote.
#[tokio::test]
async fn vcs_reports_branch_head_upstream_ahead_behind_and_commits() {
    let project = repo_with_commit();
    let root = project.path();
    let remote = tempfile::tempdir().unwrap();
    git(
        remote.path(),
        &["init", "--quiet", "--bare", "-b", "main", "."],
    );
    let remote_url = remote.path().to_str().unwrap();
    git(root, &["remote", "add", "origin", remote_url]);
    git(root, &["push", "--quiet", "-u", "origin", "main"]);
    let first = git(root, &["rev-parse", "HEAD"]);
    commit(root, "upstream only");
    git(root, &["push", "--quiet", "origin", "main"]);
    git(root, &["reset", "--quiet", "--hard", &first]);
    commit(root, "third");
    let head = commit(root, "fourth: the subject, with | and \u{1f} kept");

    let body = vcs(root).await;
    assert_eq!(body["branch"], "main");
    assert_eq!(body["head"], head);
    assert_eq!(body["upstream"], "origin/main");
    assert_eq!(body["ahead"], 2);
    assert_eq!(body["behind"], 1);
    assert_eq!(body["dirty"], json!([]));

    let commits = body["commits"].as_array().unwrap();
    assert_eq!(
        strings(&body["commits"], "subject"),
        [
            "fourth: the subject, with | and \u{1f} kept",
            "third",
            "first"
        ]
    );
    assert_eq!(commits[0]["sha"], head);
    assert_eq!(commits[2]["sha"], first);
    assert_eq!(commits[0]["author"], "repoview test");
    let date = commits[0]["date"].as_str().unwrap();
    assert!(
        date.len() >= 20 && date.as_bytes()[4] == b'-' && date.as_bytes()[10] == b'T',
        "ISO 8601 author date: {date}"
    );
    assert_eq!(
        body["remotes"],
        json!([{ "name": "origin", "url": remote_url }])
    );
    assert_eq!(
        body["worktrees"],
        json!([{
            "path": canonical(root),
            "head": head,
            "branch": "main",
            "locked": false,
            "prunable": false,
        }])
    );
}

#[tokio::test]
async fn vcs_detached_head_has_no_branch_and_no_upstream() {
    let project = repo_with_commit();
    let root = project.path();
    let head = commit(root, "second");
    git(root, &["checkout", "--quiet", "--detach", "HEAD~1"]);
    let detached = git(root, &["rev-parse", "HEAD"]);
    assert_ne!(detached, head);

    let body = vcs(root).await;
    assert_eq!(body["branch"], Value::Null);
    assert_eq!(body["head"], detached);
    assert_eq!(body["upstream"], Value::Null);
    assert_eq!(body["ahead"], Value::Null);
    assert_eq!(body["behind"], Value::Null);
    assert_eq!(strings(&body["commits"], "subject"), ["first"]);
    assert_eq!(body["worktrees"][0]["branch"], Value::Null);
    assert_eq!(body["worktrees"][0]["head"], detached);
}

#[tokio::test]
async fn vcs_unborn_branch_has_a_name_and_no_head_commits_or_tags() {
    let project = unborn();
    let root = project.path();
    fs::write(root.join("new.txt"), "x").unwrap();

    let body = vcs(root).await;
    assert_eq!(body["branch"], "main");
    assert_eq!(body["head"], Value::Null);
    assert_eq!(body["upstream"], Value::Null);
    assert_eq!(body["commits"], json!([]));
    assert_eq!(body["tags"], json!([]));
    assert_eq!(body["remotes"], json!([]));
    assert_eq!(
        body["dirty"],
        json!([{ "path": "new.txt", "status": "??", "orig_path": null }])
    );
    assert_eq!(
        body["worktrees"],
        json!([{
            "path": canonical(root),
            "head": null,
            "branch": "main",
            "locked": false,
            "prunable": false,
        }])
    );
}

/// An upstream configured on a branch whose remote-tracking ref is gone: upstream named, no
/// ahead/behind.
#[tokio::test]
async fn vcs_gone_upstream_names_it_without_counts() {
    let project = repo_with_commit();
    let root = project.path();
    git(root, &["config", "branch.main.remote", "origin"]);
    git(root, &["config", "branch.main.merge", "refs/heads/main"]);
    git(
        root,
        &["remote", "add", "origin", "/nonexistent/remote.git"],
    );

    let body = vcs(root).await;
    assert_eq!(body["upstream"], "origin/main");
    assert_eq!(body["ahead"], Value::Null);
    assert_eq!(body["behind"], Value::Null);
}

/// One dirty file per porcelain status this repository can produce: unstaged and staged
/// modification, addition, unstaged and staged deletion, rename, type change, unmerged and
/// untracked, plus a path with a space and a newline.
#[tokio::test]
async fn vcs_lists_each_dirty_file_with_its_porcelain_status() {
    let project = unborn();
    let root = project.path();
    for name in ["m", "s", "d1", "d2", "r", "t", "c"] {
        fs::write(root.join(name), format!("{name}\nline two\nline three\n")).unwrap();
    }
    git(root, &["add", "."]);
    commit(root, "base");
    git(root, &["checkout", "--quiet", "-b", "other"]);
    fs::write(root.join("c"), "other\n").unwrap();
    git(root, &["commit", "--quiet", "-am", "other side"]);
    git(root, &["checkout", "--quiet", "main"]);
    fs::write(root.join("c"), "main\n").unwrap();
    git(root, &["commit", "--quiet", "-am", "main side"]);
    let merge = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(["merge", "--quiet", "other"])
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(!merge.status.success(), "the merge conflicts");

    fs::write(root.join("m"), "changed\n").unwrap();
    fs::write(root.join("s"), "changed\n").unwrap();
    git(root, &["add", "s"]);
    fs::write(root.join("a"), "added\n").unwrap();
    git(root, &["add", "a"]);
    fs::remove_file(root.join("d1")).unwrap();
    git(root, &["rm", "--quiet", "d2"]);
    git(root, &["mv", "r", "r2"]);
    fs::remove_file(root.join("t")).unwrap();
    std::os::unix::fs::symlink("m", root.join("t")).unwrap();
    fs::write(root.join("u"), "untracked\n").unwrap();
    fs::write(root.join("with space\nand newline"), "odd\n").unwrap();

    let body = vcs(root).await;
    let mut dirty = body["dirty"].as_array().unwrap().clone();
    dirty.sort_by_key(|entry| entry["path"].as_str().unwrap().to_owned());
    let entry =
        |path: &str, status: &str| json!({ "path": path, "status": status, "orig_path": null });
    assert_eq!(
        dirty,
        [
            entry("a", "A."),
            entry("c", "UU"),
            entry("d1", ".D"),
            entry("d2", "D."),
            entry("m", ".M"),
            json!({ "path": "r2", "status": "R.", "orig_path": "r" }),
            entry("s", "M."),
            entry("t", ".T"),
            entry("u", "??"),
            entry("with space\nand newline", "??"),
        ]
    );
}

#[tokio::test]
async fn vcs_tags_are_newest_first_peeled_to_their_commit_and_capped_at_thirty() {
    let project = unborn();
    let root = project.path();
    let dated = |date: &'static str| [("GIT_COMMITTER_DATE", date), ("GIT_AUTHOR_DATE", date)];
    git_env(
        root,
        &["commit", "--quiet", "--allow-empty", "-m", "old"],
        &dated("2020-01-01T00:00:00Z"),
    );
    let old = git(root, &["rev-parse", "HEAD"]);
    for index in 0..30 {
        git(root, &["tag", &format!("bulk-{index:02}")]);
    }
    git_env(
        root,
        &["tag", "-a", "annotated", "-m", "an annotated tag"],
        &dated("2021-06-01T00:00:00Z"),
    );
    git_env(
        root,
        &["commit", "--quiet", "--allow-empty", "-m", "new"],
        &dated("2022-01-01T00:00:00Z"),
    );
    let new = git(root, &["rev-parse", "HEAD"]);
    git(root, &["tag", "newest"]);

    let body = vcs(root).await;
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 30, "newest 30 of 32");
    assert_eq!(tags[0]["name"], "newest");
    assert_eq!(tags[0]["sha"], new);
    assert!(
        tags[0]["date"].as_str().unwrap().starts_with("2022-01-01"),
        "{}",
        tags[0]["date"]
    );
    assert_eq!(tags[1]["name"], "annotated");
    assert_eq!(tags[1]["sha"], old, "an annotated tag shows its commit");
    assert!(tags[1]["date"].as_str().unwrap().starts_with("2021-06-01"));
    assert!(tags[2]["name"].as_str().unwrap().starts_with("bulk-"));
}

#[tokio::test]
async fn vcs_commits_are_the_last_fifty() {
    let project = unborn();
    let root = project.path();
    for index in 0..55 {
        commit(root, &format!("commit {index}"));
    }
    let body = vcs(root).await;
    let subjects = strings(&body["commits"], "subject");
    assert_eq!(subjects.len(), 50);
    assert_eq!(subjects[0], "commit 54");
    assert_eq!(subjects[49], "commit 5");
}

#[tokio::test]
async fn vcs_remote_urls_lose_their_userinfo() {
    let project = repo_with_commit();
    let root = project.path();
    for (name, url) in [
        ("a-token", "https://x-access-token:abc@github.com/o/r.git"),
        ("b-user-only", "https://ghp_secret123@github.com/o/r"),
        (
            "c-ssh-password",
            "ssh://user:hunter2@host.example:2222/r.git",
        ),
        ("d-scp", "git@github.com:o/r.git"),
        ("e-scp-password", "user:hunter2@host.example:o/r.git"),
        ("f-local", "/srv/git/r@2.git"),
    ] {
        git(root, &["remote", "add", name, url]);
    }
    git(
        root,
        &[
            "remote",
            "set-url",
            "--push",
            "a-token",
            "https://x-access-token:pushsecret@github.com/o/r.git",
        ],
    );
    let path = bin_dir(&["git"]);
    let (status, raw) = get_raw(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(raw).unwrap();
    for secret in [
        "abc@",
        "x-access-token",
        "ghp_secret123",
        "hunter2",
        "pushsecret",
    ] {
        assert!(!text.contains(secret), "{secret} leaked: {text}");
    }
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        body["remotes"],
        json!([
            { "name": "a-token", "url": "https://github.com/o/r.git" },
            { "name": "b-user-only", "url": "https://github.com/o/r" },
            { "name": "c-ssh-password", "url": "ssh://host.example:2222/r.git" },
            { "name": "d-scp", "url": "git@github.com:o/r.git" },
            { "name": "e-scp-password", "url": "host.example:o/r.git" },
            { "name": "f-local", "url": "/srv/git/r@2.git" },
        ])
    );
}

#[tokio::test]
async fn vcs_lists_the_main_and_every_linked_worktree() {
    let project = repo_with_commit();
    let root = project.path();
    let head = git(root, &["rev-parse", "HEAD"]);
    let linked = tempfile::tempdir().unwrap();
    let feature = linked.path().join("feature");
    let detached = linked.path().join("detached");
    let gone = linked.path().join("gone");
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature",
            feature.to_str().unwrap(),
        ],
    );
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            detached.to_str().unwrap(),
        ],
    );
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "gone",
            gone.to_str().unwrap(),
        ],
    );
    git(root, &["worktree", "lock", detached.to_str().unwrap()]);
    fs::remove_dir_all(&gone).unwrap();

    let body = vcs(root).await;
    let worktree = |path: String, branch: Value, locked: bool, prunable: bool| {
        json!({
            "path": path,
            "head": head,
            "branch": branch,
            "locked": locked,
            "prunable": prunable,
        })
    };
    // Git lists the main worktree first, then the linked ones by path.
    assert_eq!(
        body["worktrees"],
        json!([
            worktree(canonical(root), json!("main"), false, false),
            worktree(canonical(&detached), Value::Null, true, false),
            worktree(canonical(&feature), json!("feature"), false, false),
            worktree(
                format!("{}/gone", canonical(linked.path())),
                json!("gone"),
                false,
                true
            ),
        ])
    );
}

/// A `git` in `dir` wrapping the real one. `refuse_worktree_z` makes `worktree list … -z` exit 129
/// as Git 2.34 does; each subcommand in `failing` exits 1 with `fatal: <subcommand> broken`.
fn wrapped_git(dir: &Path, refuse_worktree_z: bool, failing: &[&str]) {
    use std::os::unix::fs::PermissionsExt;
    let mut script = String::from("#!/bin/sh\n");
    if refuse_worktree_z {
        script.push_str(concat!(
            "if [ \"$1\" = worktree ]; then\n",
            "  for arg in \"$@\"; do\n",
            "    if [ \"$arg\" = -z ]; then echo \"error: unknown switch \\`z'\" >&2; exit 129; fi\n",
            "  done\n",
            "fi\n",
        ));
    }
    for name in failing {
        script.push_str(&format!(
            "if [ \"$1\" = {name} ]; then echo 'fatal: {name} broken' >&2; exit 1; fi\n"
        ));
    }
    script.push_str(&format!("exec '{}' \"$@\"\n", real_tool("git").display()));
    let path = dir.join("git");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("--version").output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return,
        }
    }
    panic!("git wrapper stayed busy");
}

/// Correction round 2: a Git without `worktree list -z` (before 2.36) still lists every worktree,
/// from the newline-separated porcelain, with branch, detached, locked and prunable.
#[tokio::test]
async fn vcs_lists_worktrees_on_a_git_without_worktree_list_z() {
    let project = repo_with_commit();
    let root = project.path();
    let head = git(root, &["rev-parse", "HEAD"]);
    let linked = tempfile::tempdir().unwrap();
    let detached = linked.path().join("detached");
    let gone = linked.path().join("gone");
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            detached.to_str().unwrap(),
        ],
    );
    git(
        root,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "gone",
            gone.to_str().unwrap(),
        ],
    );
    git(root, &["worktree", "lock", detached.to_str().unwrap()]);
    fs::remove_dir_all(&gone).unwrap();
    let path = tempfile::tempdir().unwrap();
    wrapped_git(path.path(), true, &[]);

    let (status, body) = get(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let worktree = |path: String, branch: Value, locked: bool, prunable: bool| {
        json!({
            "path": path,
            "head": head,
            "branch": branch,
            "locked": locked,
            "prunable": prunable,
        })
    };
    assert_eq!(
        body["worktrees"],
        json!([
            worktree(canonical(root), json!("main"), false, false),
            worktree(canonical(&detached), Value::Null, true, false),
            worktree(
                format!("{}/gone", canonical(linked.path())),
                json!("gone"),
                false,
                true
            ),
        ])
    );
    assert_eq!(body["worktrees_error"], Value::Null);
}

/// Correction round 2: one block's failure does not fail the route. When `git tag`,
/// `git remote` or `git worktree` fails, `/api/vcs` is still 200 with status and commits, and
/// that block is empty with its error.
#[tokio::test]
async fn vcs_answers_when_tags_remotes_or_worktrees_fail() {
    let project = repo_with_commit();
    let root = project.path();
    let head = git(root, &["rev-parse", "HEAD"]);
    git(root, &["tag", "v1"]);
    git(root, &["remote", "add", "origin", "/srv/r.git"]);
    let path = tempfile::tempdir().unwrap();
    wrapped_git(path.path(), false, &["tag", "remote", "worktree"]);

    let (status, body) = get(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["head"], head);
    assert_eq!(strings(&body["commits"], "subject"), ["first"]);
    for block in ["tags", "remotes", "worktrees"] {
        assert_eq!(body[block], json!([]), "{block}");
        let name = block.trim_end_matches('s');
        assert_eq!(
            body[format!("{block}_error")].as_str().map(str::trim),
            Some(format!("fatal: {name} broken").as_str()),
            "{block}: {body}"
        );
    }
}

/// Without failures every block's error is present and `null`.
#[tokio::test]
async fn vcs_block_errors_are_null_when_every_block_reads() {
    let project = repo_with_commit();
    let body = vcs(project.path()).await;
    for block in ["tags_error", "remotes_error", "worktrees_error"] {
        assert_eq!(body.get(block), Some(&Value::Null), "{block}: {body}");
    }
}

#[tokio::test]
async fn vcs_outside_git_is_404_absent_with_the_tool_shape() {
    let project = tempfile::tempdir().unwrap();
    let path = bin_dir(&["git"]);
    let (status, body) = get(project.path(), path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["availability"], "Absent");
    assert_eq!(body["tool"], "git");
}

#[tokio::test]
async fn vcs_without_git_on_path_is_503_tool_missing() {
    let project = repo_with_commit();
    let empty = tempfile::tempdir().unwrap();
    let (status, body) = get(project.path(), empty.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["availability"], "ToolMissing");
    assert_eq!(body["tool"], "git");
    assert_eq!(body["diagnostic"], "git not found on PATH");
}

#[tokio::test]
async fn every_route_needs_the_token() {
    let project = repo_with_commit();
    for uri in ["/api/vcs", "/api/docs", "/api/docs/README.md"] {
        let response = tower::ServiceExt::oneshot(
            router(state()).layer(Extension(Env::new(project.path()))),
            Request::get(uri)
                .header(header::HOST, HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
    }
}

/// Acceptance 3: `.git/index` is unchanged after every request, even with a tracked file whose
/// stat data no longer matches the index (which a plain `git status` would refresh and write).
#[tokio::test]
async fn dot_git_index_is_unchanged_after_every_request() {
    let project = unborn();
    let root = project.path();
    fs::write(root.join("README.md"), "# readme\n").unwrap();
    git(root, &["add", "."]);
    commit(root, "base");
    let stale = SystemTime::now() - Duration::from_secs(3600);
    fs::File::options()
        .write(true)
        .open(root.join("README.md"))
        .unwrap()
        .set_modified(stale)
        .unwrap();
    let index = root.join(".git/index");
    let read = || {
        (
            fs::read(&index).unwrap(),
            fs::metadata(&index).unwrap().modified().unwrap(),
        )
    };
    let before = read();
    let path = bin_dir(&["git"]);
    for uri in ["/api/vcs", "/api/docs", "/api/docs/README.md"] {
        let (status, _) = get_raw(root, path.path(), uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(read() == before, ".git/index changed after {uri}");
    }
    // The case bites: a plain `git status` does rewrite this index.
    git(root, &["status", "--porcelain"]);
    assert!(read() != before, "the fixture's index was not stale");
}

// ---- /api/docs -----------------------------------------------------------------------------

#[tokio::test]
async fn docs_lists_the_four_documents_with_presence() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("README.md"), "# r\n").unwrap();
    fs::write(root.join("CHANGELOG.md"), "# c\n").unwrap();
    fs::create_dir(root.join("STATUS.md")).unwrap();
    let path = bin_dir(&[]);
    let (status, body) = get(root, path.path(), "/api/docs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!([
            { "name": "README.md", "present": true },
            { "name": "AGENTS.md", "present": false },
            { "name": "STATUS.md", "present": false },
            { "name": "CHANGELOG.md", "present": true },
        ])
    );
}

#[tokio::test]
async fn doc_returns_the_file_content() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let text = "# Agents\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n<script>x</script>\n";
    fs::write(root.join("AGENTS.md"), text).unwrap();
    let path = bin_dir(&[]);
    let (status, body) = get(root, path.path(), "/api/docs/AGENTS.md").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "name": "AGENTS.md", "markdown": text }));
}

#[tokio::test]
async fn doc_outside_the_four_names_or_absent_is_404() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("LICENSE"), "secret-ish\n").unwrap();
    fs::write(root.join("README.md"), "# r\n").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/README.md"), "# nested\n").unwrap();
    let path = bin_dir(&[]);
    for uri in [
        "/api/docs/LICENSE",
        "/api/docs/readme.md",
        "/api/docs/STATUS.md",
        "/api/docs/sub%2FREADME.md",
        "/api/docs/..%2FREADME.md",
        "/api/docs/README.md%00",
    ] {
        let (status, raw) = get_raw(root, path.path(), uri).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{uri}: {}",
            String::from_utf8_lossy(&raw)
        );
    }
}

/// Correction round 1: a document is served only as a regular file. A symlink is absent and 404
/// wherever it points: outside the project, inside it, or into `.git` (where `config` holds the
/// remote URLs `/api/vcs` redacts). A FIFO is absent too, and reading the list does not block.
#[tokio::test]
async fn doc_that_is_not_a_regular_file_is_absent() {
    let project = repo_with_commit();
    let root = project.path();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.md"), "secret\n").unwrap();
    std::os::unix::fs::symlink(outside.path().join("secret.md"), root.join("README.md")).unwrap();
    fs::create_dir(root.join("docs")).unwrap();
    fs::write(root.join("docs/agents.md"), "# inside\n").unwrap();
    std::os::unix::fs::symlink("docs/agents.md", root.join("AGENTS.md")).unwrap();
    std::os::unix::fs::symlink(".git/config", root.join("STATUS.md")).unwrap();
    let fifo = Command::new("mkfifo")
        .arg(root.join("CHANGELOG.md"))
        .status()
        .unwrap();
    assert!(fifo.success());
    let path = bin_dir(&["git"]);
    let (status, list) = get(root, path.path(), "/api/docs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        list,
        json!([
            { "name": "README.md", "present": false },
            { "name": "AGENTS.md", "present": false },
            { "name": "STATUS.md", "present": false },
            { "name": "CHANGELOG.md", "present": false },
        ])
    );
    for name in ["README.md", "AGENTS.md", "STATUS.md", "CHANGELOG.md"] {
        let (status, raw) = get_raw(root, path.path(), &format!("/api/docs/{name}")).await;
        let text = String::from_utf8_lossy(&raw);
        assert_eq!(status, StatusCode::NOT_FOUND, "{name}: {text}");
        assert!(
            !text.contains("secret") && !text.contains("[core]"),
            "{name}: {text}"
        );
    }
}

/// Correction round 1: a project root that is itself the Git directory (a bare repository) serves
/// no document from it, even a regular file.
#[tokio::test]
async fn doc_inside_the_git_directory_is_absent() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    git(root, &["init", "--quiet", "--bare", "-b", "main", "."]);
    fs::write(root.join("README.md"), "# in the git dir\n").unwrap();
    let path = bin_dir(&["git"]);
    let (_, list) = get(root, path.path(), "/api/docs").await;
    assert_eq!(list[0], json!({ "name": "README.md", "present": false }));
    let (status, _) = get_raw(root, path.path(), "/api/docs/README.md").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Outside Git, or without `git` on `PATH`, regular documents are still served.
#[tokio::test]
async fn doc_is_served_outside_git_and_without_git() {
    let plain = tempfile::tempdir().unwrap();
    fs::write(plain.path().join("README.md"), "# plain\n").unwrap();
    let repo = repo_with_commit();
    fs::write(repo.path().join("README.md"), "# repo\n").unwrap();
    let with_git = bin_dir(&["git"]);
    let without_git = bin_dir(&[]);
    for (root, path, text) in [
        (plain.path(), with_git.path(), "# plain\n"),
        (repo.path(), without_git.path(), "# repo\n"),
        (repo.path(), with_git.path(), "# repo\n"),
    ] {
        let (status, body) = get(root, path, "/api/docs/README.md").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["markdown"], text);
    }
}

/// Correction round 1: `/api/tasks` is gone; the route is the `/api` 404.
#[tokio::test]
async fn tasks_route_is_gone() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("Taskfile.yml"), "version: '3'\n").unwrap();
    let path = bin_dir(&[]);
    let (status, _) = get_raw(project.path(), path.path(), "/api/tasks").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn doc_over_one_mebibyte_is_413() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("README.md"), vec![b'a'; 1024 * 1024]).unwrap();
    fs::write(root.join("CHANGELOG.md"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    let path = bin_dir(&[]);
    let (status, body) = get(root, path.path(), "/api/docs/README.md").await;
    assert_eq!(status, StatusCode::OK, "exactly 1 MiB is served");
    assert_eq!(body["markdown"].as_str().unwrap().len(), 1024 * 1024);
    let (status, _) = get_raw(root, path.path(), "/api/docs/CHANGELOG.md").await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}
