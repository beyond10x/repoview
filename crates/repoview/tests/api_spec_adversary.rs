//! Adversary cases for story:spec-pages acceptance 1: what happens before a `{root}` is known to
//! be a detected root.

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
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:7480";

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

struct Fixture {
    _base: TempDir,
    project: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new(roots: &[&str]) -> Fixture {
        let base = tempfile::tempdir().unwrap();
        let project = base.path().join("project");
        let bin = base.path().join("bin");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(base.path().join("x")).unwrap();
        fs::write(base.path().join("x/system.yaml"), "format: ess/20\n").unwrap();
        for root in roots {
            let dir = project.join(root);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("system.yaml"), "format: ess/20\n").unwrap();
        }
        Fixture {
            _base: base,
            project,
            bin,
        }
    }

    /// A stub `ess` that appends every invocation, `--version` included, to `every-call.log`.
    fn with_logging_stub(self) -> Fixture {
        let path = self.bin.join("ess");
        let body = r#"#!/bin/sh
printf '%s\n' "$*" >> "${0%/*}/every-call.log"
if [ "$1" = "--version" ]; then printf 'ess 7.7.7\n'; exit 0; fi
printf '{"valid":true}\n'
"#;
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        wait_executable(&path);
        let _ = fs::remove_file(self.bin.join("every-call.log"));
        self
    }

    fn every_call(&self) -> Vec<String> {
        fs::read_to_string(self.bin.join("every-call.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn app(&self) -> Router {
        router(state()).layer(Extension(Env::with_path(&self.project, &self.bin)))
    }
}

fn wait_executable(path: &Path) {
    for _ in 0..200 {
        match Command::new(path).arg("--version").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return,
        }
    }
    panic!("stub {} stayed busy", path.display());
}

async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::get(uri)
        .header(header::HOST, HOST)
        .header(TOKEN_HEADER, token())
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, value)
}

/// Acceptance 1: "`{root}` … must be one of the detected roots exactly, else 404 and no process
/// starts." The existing case only checks that no `ess specify` was logged; the stub there does not
/// log `--version`.
#[tokio::test]
async fn an_undetected_root_starts_no_process_at_all() {
    let fixture = Fixture::new(&["ess"]).with_logging_stub();
    for uri in [
        "/api/spec/roots/../x/ir",
        "/api/spec/roots/..%2Fx/graph",
        "/api/spec/roots/nope/mermaid",
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
    }
    assert_eq!(fixture.every_call(), Vec::<String>::new());
}

/// Acceptance 1: an undetected `{root}` is 404. Detection of roots is a file walk that needs no
/// `ess`, yet without `ess` on `PATH` the answer is 503.
#[tokio::test]
async fn an_undetected_root_is_404_even_when_ess_is_missing() {
    let fixture = Fixture::new(&["ess"]);
    for uri in ["/api/spec/roots/../x/ir", "/api/spec/roots/nope/ir"] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
    }
}
