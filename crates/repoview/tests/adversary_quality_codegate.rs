//! Adversary cases for story:quality-codegate acceptance 4: the Overview `quality` card (the
//! snapshot, read afresh on every request) and the Quality page (`/api/quality`) must report the
//! same binary and version for the life of one `repoview open`, including after `codegate` on
//! `PATH` is upgraded while the server runs.

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
use common::bin_dir;
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use repoview_sources::{Env, read_all};
use serde_json::{Value, json};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:7480";

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

async fn get_quality(app: &Router, token: &str) -> (StatusCode, Value) {
    let request = Request::get("/api/quality")
        .header(header::HOST, HOST)
        .header(TOKEN_HEADER, token)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// codegate 0.3.0 is on `PATH` when the Quality page is first opened; it is then upgraded in
/// place (here: the version it reports changes) while `repoview open` keeps running. The
/// Overview card reads the snapshot afresh and names the new version; the Quality page must not
/// go on naming the old one.
#[tokio::test(flavor = "multi_thread")]
async fn after_an_in_place_upgrade_the_api_and_the_snapshot_still_agree() {
    let bin = bin_dir(&["cat"]);
    let version = bin.path().join("version");
    fs::write(&version, "0.3.0").unwrap();
    write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
case "$1" in
  --version) printf 'codegate %s\n' "$(cat '{version}')" ;;
  --help) printf 'Usage: codegate <COMMAND>\n\nCommands:\n  evaluate  \n  help  x\n' ;;
  *) exit 64 ;;
esac"#,
            version = version.display()
        ),
    );
    let dir = tempfile::tempdir().unwrap();
    let env = Env::with_path(dir.path(), bin.path());
    let token = new_token();
    let app = router(AppState {
        token: token.clone(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    })
    .layer(Extension(env.clone()));

    let (status, first) = get_quality(&app, &token).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["tool_version"], "0.3.0");

    fs::write(&version, "0.4.0").unwrap();

    let (status, page) = get_quality(&app, &token).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    let card = read_all(&env)
        .into_iter()
        .find(|section| section.source_id == "quality")
        .unwrap();
    assert_eq!(
        (page["tool_path"].clone(), page["tool_version"].clone()),
        (json!(card.tool_path), json!(card.tool_version)),
        "the Quality page and the Overview card name different codegates"
    );
}
