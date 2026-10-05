//! Adversary, story:plan-pages: ids that `aep` itself accepts and that real stores hold.
//!
//! `aep_domain::ArtifactId::new` (aep 0.68.0) admits `[A-Za-z0-9._/-]` on either side of the colon,
//! and every vision in the beyond10x stores that carry one besides repoview's is `vision:O<n>`
//! (aep, canon, commission, engineering-protocols, entity-runtime, ess, governor, intake, loom).
//! The Tree page roots are those visions; their Artifact page reads `/api/plan/artifacts/{id}`.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::json;
use tower::ServiceExt;

fn token() -> String {
    "a".repeat(64)
}

fn state() -> AppState {
    AppState {
        token: token(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new().with("index.html", "<title>spa</title>")),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    }
}

fn stub(dir: &Path, body: &str) {
    let path = dir.join("aep");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("--probe").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return,
        }
    }
    panic!("stub stayed busy");
}

#[tokio::test]
async fn a_vision_id_as_aep_writes_it_reaches_aep() {
    let project = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    stub(bin.path(), "printf '{}'");
    let env = Env::with_path(project.path(), bin.path().as_os_str());
    for id in ["vision:O2", "story:a_b"] {
        let request = Request::get(format!("/api/plan/artifacts/{id}"))
            .header(header::HOST, "127.0.0.1:7480")
            .header(TOKEN_HEADER, token())
            .body(Body::empty())
            .unwrap();
        let response = router(state())
            .layer(Extension(env.clone()))
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "{id}: {}",
            String::from_utf8_lossy(&body)
        );
    }
}
