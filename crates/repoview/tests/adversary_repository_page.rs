//! Adversary cases for story:repository-page: `/api/tasks` executing repository code, `git log`
//! field splitting, and a document symlinked into `.git`.

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::{Value, json};

use common::bin_dir;

fn token() -> String {
    "a".repeat(64)
}

async fn get_raw(root: &Path, path: &Path, uri: &str) -> (StatusCode, Vec<u8>) {
    let state = AppState {
        token: token(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    };
    let response = tower::ServiceExt::oneshot(
        router(state).layer(Extension(Env::with_path(root, path))),
        Request::get(uri)
            .header(header::HOST, "127.0.0.1:7480")
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

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.email=test@example.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// An executable `/bin/sh` stub `name` in `dir` that records, in the working directory, that it
/// was started. Waits out ETXTBSY from a concurrently forked test thread.
fn recording_stub(dir: &Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    fs::write(
        &path,
        format!("#!/bin/sh\n: > \"$PWD/{name}-was-started\"\n"),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    for _ in 0..200 {
        match Command::new(&path).current_dir(scratch.path()).output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            _ => return,
        }
    }
    panic!("stub {name} stayed busy");
}

/// Opening the Repository page requested `/api/tasks`. With the real `task`, listing evaluates the
/// Taskfile's dynamic variables (`vars: X: sh: …`) whenever it declares `dotenv:`, so a cloned
/// repository ran its own shell command on page load.
///
/// Correction round 1 (coordinator decision): the route is removed; a later story reads
/// `Taskfile.yml` without executing it. So `GET /api/tasks` is 404 and starts no process: a
/// `task` on `PATH` is never run and the probe Taskfile's `sh:` command never fires.
#[tokio::test]
async fn tasks_listing_executes_no_taskfile_shell_command() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(
        root.join("Taskfile.yml"),
        concat!(
            "version: '3'\n",
            "vars:\n",
            "  PROBE:\n",
            "    sh: touch executed-by-listing\n",
            "dotenv: ['.env']\n",
            "tasks:\n",
            "  check:\n",
            "    desc: The gate\n",
            "    cmds: [echo hi]\n",
        ),
    )
    .unwrap();
    // The server runs with the user's PATH; `touch` stands in for it here, and `task` is a stub
    // that leaves `task-was-started` in the project if anything runs it.
    let path = bin_dir(&["touch"]);
    recording_stub(path.path(), "task");
    let (status, body) = get_raw(root, path.path(), "/api/tasks").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(
        !root.join("task-was-started").exists(),
        "GET /api/tasks started `task`"
    );
    assert!(
        !root.join("executed-by-listing").exists(),
        "GET /api/tasks ran the Taskfile's `sh:` variable command in the project"
    );
}

/// Git stores an author name containing U+001F verbatim; it must not shift `date` and
/// `subject`.
#[tokio::test]
async fn vcs_author_with_a_unit_separator_keeps_its_date_and_subject() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    git(root, &["init", "--quiet", "-b", "main"]);
    git(
        root,
        &[
            "-c",
            "user.name=Eve\u{1f}Forged",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "the subject",
        ],
    );
    let path = bin_dir(&["git"]);
    let (status, raw) = get_raw(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_slice(&raw).unwrap();
    let commit = &body["commits"][0];
    assert_eq!(commit["author"], "Eve\u{1f}Forged", "{commit}");
    assert_eq!(commit["subject"], "the subject", "{commit}");
    assert!(
        commit["date"].as_str().unwrap().starts_with("20"),
        "{commit}"
    );
}

/// A repository can commit `README.md -> .git/config` and a clone checks it out as that symlink.
/// The target is inside the project, so `/api/docs/README.md` serves `.git/config`, and with it
/// the remote URL credentials `/api/vcs` redacts.
#[tokio::test]
async fn doc_symlinked_into_dot_git_does_not_serve_remote_credentials() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    git(root, &["init", "--quiet", "-b", "main"]);
    git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://x-access-token:abc@github.com/o/r.git",
        ],
    );
    std::os::unix::fs::symlink(".git/config", root.join("README.md")).unwrap();
    let path = bin_dir(&["git"]);
    let (_, raw) = get_raw(root, path.path(), "/api/docs/README.md").await;
    let text = String::from_utf8_lossy(&raw);
    assert!(
        !text.contains("x-access-token:abc@"),
        "remote credentials left the server through /api/docs: {text}"
    );
}
