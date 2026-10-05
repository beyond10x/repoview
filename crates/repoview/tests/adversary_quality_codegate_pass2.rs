//! Adversary cases, pass 2, for story:quality-codegate correction round 1: `--help` is read again
//! only when the located binary's identity (path, `--version` text, size, mtime) changes.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::{Extension, Router};
use common::{bin_dir, real_tool};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tower::ServiceExt;

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("probe-busy").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return path,
        }
    }
    panic!("stub {name} stayed busy");
}

fn app(root: &Path, path: &Path) -> (Router, String) {
    let token = new_token();
    let state = AppState {
        token: token.clone(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    };
    (
        router(state).layer(Extension(Env::with_path(root, path))),
        token,
    )
}

async fn get_quality(app: &Router, token: &str) -> (StatusCode, Value) {
    let request = Request::get("/api/quality")
        .header(header::HOST, "127.0.0.1:7480")
        .header(TOKEN_HEADER, token)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

/// A rebuilt codegate (same version, now offering `assess`) is installed at the moment the
/// server's `codegate --help` runs, i.e. after `--version` identified the old binary and before
/// the server takes the binary's size and mtime. `Identity::of` is taken after `--help`
/// (`read_offer`), so the cache pairs the old binary's commands with the new binary's identity,
/// and every later request, which locates the new binary, matches that identity and is answered
/// with the old commands for the rest of the server run. Taking the identity before `--help`
/// would make the next request read `--help` again.
#[tokio::test(flavor = "multi_thread")]
async fn a_codegate_replaced_while_its_help_runs_is_probed_again_on_the_next_request() {
    let staging = bin_dir(&[]);
    let rebuilt = write_stub(
        staging.path(),
        "codegate",
        r#"[ "$1" = probe-busy ] && exit 0
case "$1" in
  --version) printf 'codegate 0.3.0\n' ;;
  --help) printf 'Commands:\n  evaluate  \n  assess    Assess the source\n  help      Print help\n' ;;
  *) exit 64 ;;
esac"#,
    );
    let bin = bin_dir(&[]);
    let installed = bin.path().join("codegate");
    write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
case "$1" in
  --version) printf 'codegate 0.3.0\n' ;;
  --help) '{mv}' '{rebuilt}' '{installed}'; printf 'Commands:\n  evaluate  \n  help      Print help\n' ;;
  *) exit 64 ;;
esac"#,
            mv = real_tool("mv").display(),
            rebuilt = rebuilt.display(),
            installed = installed.display(),
        ),
    );
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), bin.path());

    let (status, first) = get_quality(&app, &token).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["commands"], json!(["evaluate"]));
    assert!(!rebuilt.exists(), "the rebuilt codegate was not installed");
    assert!(
        fs::read_to_string(&installed)
            .unwrap()
            .contains("Assess the source"),
        "the rebuilt codegate is not the one on PATH"
    );

    let (status, second) = get_quality(&app, &token).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(
        second["commands"],
        json!(["evaluate", "assess"]),
        "the codegate on PATH now offers assess; the answer still lists the replaced binary's commands"
    );
}
