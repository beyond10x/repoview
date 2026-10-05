//! story:quality-codegate correction round 2: the colour environment every `codegate` child gets.
//! clap colours output under any set `CLICOLOR_FORCE`, `0` included, unless `NO_COLOR` is set,
//! so the child must not inherit `CLICOLOR_FORCE` at all. Its own test binary, because it sets
//! process environment variables.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use common::bin_dir;
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

/// `--help` lists one command whose name spells out the child's colour environment.
const ENV_ECHO: &str = r#"[ "$1" = probe-busy ] && exit 0
case "$1" in
  --version) printf 'codegate 0.3.0\n' ;;
  --help) printf 'Commands:\n  force=%s,no_color=%s,clicolor=%s,term=%s  \n' \
    "${CLICOLOR_FORCE-unset}" "${NO_COLOR-unset}" "${CLICOLOR-unset}" "${TERM-unset}" ;;
  *) exit 64 ;;
esac"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_codegate_child_gets_no_clicolor_force_and_colour_off() {
    // SAFETY: the only test in this binary, set before anything in it reads the environment or
    // spawns a child.
    unsafe {
        std::env::set_var("CLICOLOR_FORCE", "1");
        std::env::set_var("CLICOLOR", "1");
        std::env::set_var("TERM", "xterm-256color");
        std::env::remove_var("NO_COLOR");
    }
    let bin = bin_dir(&[]);
    write_stub(bin.path(), "codegate", ENV_ECHO);
    let dir = tempfile::tempdir().unwrap();
    let token = new_token();
    let app = router(AppState {
        token: token.clone(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    })
    .layer(Extension(Env::with_path(dir.path(), bin.path())));
    let request = Request::get("/api/quality")
        .header(header::HOST, "127.0.0.1:7480")
        .header(TOKEN_HEADER, &token)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        body["commands"],
        json!(["force=unset,no_color=1,clicolor=0,term=dumb"]),
        "{body}"
    );
}
