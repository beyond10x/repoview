//! Adversary cases for story:repository-page, pass 2: `/api/vcs` against a Git release that
//! predates `git worktree list -z`.

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

use common::{git, git_repo_with_commit, real_tool};

fn token() -> String {
    "b".repeat(64)
}

async fn get(root: &Path, path: &Path, uri: &str) -> (StatusCode, Value) {
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
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, value)
}

/// A `git` on `dir` that behaves as Git 2.34.1 (Ubuntu 22.04 LTS) does for `worktree list -z`:
/// `-z` arrived for that subcommand in Git 2.36, and 2.34.1 answers
/// "error: unknown switch `z'" with exit status 129 (observed in `ubuntu:22.04`,
/// `git version 2.34.1`). Every other invocation is the real `git`.
fn git_2_34(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("git");
    let script = format!(
        concat!(
            "#!/bin/sh\n",
            "if [ \"$1\" = worktree ] && [ \"$2\" = list ]; then\n",
            "  for arg in \"$@\"; do\n",
            "    if [ \"$arg\" = -z ]; then\n",
            "      echo \"error: unknown switch \\`z'\" >&2\n",
            "      exit 129\n",
            "    fi\n",
            "  done\n",
            "fi\n",
            "exec '{}' \"$@\"\n",
        ),
        real_tool("git").display()
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    // A concurrently forked test thread can hold the new file open for writing (ETXTBSY).
    for _ in 0..200 {
        match Command::new(&path).arg("--version").output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            _ => return,
        }
    }
    panic!("git stub stayed busy");
}

/// The acceptance names `git worktree list --porcelain`, which Git 2.34 supports. The route runs
/// `git worktree list --porcelain -z` and propagates its failure with `?`, so on Git 2.34 the
/// whole `/api/vcs` is 503 and the page loses status, commits, tags, remotes and worktrees.
#[tokio::test]
async fn vcs_answers_on_a_git_without_worktree_list_z() {
    let project = git_repo_with_commit();
    let root = project.path();
    let head = git(root, &["rev-parse", "HEAD"]);
    let path = tempfile::tempdir().unwrap();
    git_2_34(path.path());
    let (status, body) = get(root, path.path(), "/api/vcs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["head"], json!(head), "{body}");
    assert_eq!(body["commits"].as_array().map(Vec::len), Some(1), "{body}");
    let canonical = fs::canonicalize(root).unwrap();
    assert_eq!(
        body["worktrees"][0]["path"],
        json!(canonical.to_str().unwrap()),
        "{body}"
    );
}
