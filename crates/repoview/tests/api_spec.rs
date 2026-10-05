//! story:spec-pages acceptance 1: `/api/spec/*` against a stub `ess` on an explicit `PATH`.

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

const PORT: u16 = 7480;
const HOST: &str = "127.0.0.1:7480";

fn token() -> String {
    "a".repeat(64)
}

fn state() -> AppState {
    AppState {
        token: token(),
        port: PORT,
        assets: Arc::new(MemoryAssets::new().with("index.html", "<title>spa</title>")),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    }
}

/// The stub `ess`: `--version`, then `specify validate|compile|graph --path=<root> --format
/// json|mermaid`. Every call, `--version` included, is appended to `calls.log` beside the stub as
/// one line of arguments. A root called `bad` is refused by validate, a root called `broken` makes
/// compile fail with stderr.
const STUB: &str = r#"
log="${0%/*}/calls.log"
printf '%s\n' "$*" >> "$log"
if [ "$1" = "--version" ]; then printf 'ess 7.7.7\n'; exit 0; fi
root="${3#--path=}"
case "$2" in
  validate)
    if [ "$root" = "bad" ]; then
      printf '{"compiled":false,"files_read":1,"problems":["system.yaml: refused for test"],"diagnostics":[]}\n'
      exit 1
    fi
    printf '{"valid":true,"system":"sys-%s","version":"v1","files_read":2}\n' "$root"
    ;;
  compile)
    if [ "$root" = "broken" ]; then printf 'boom\n' >&2; exit 4; fi
    printf '{"system":"sys","argv":"%s","entities":{}}\n' "$*"
    ;;
  graph)
    if [ "$5" = "mermaid" ]; then printf 'flowchart TB\n    a --> b\n'; exit 0; fi
    printf '{"system":"sys","argv":"%s","groups":[],"nodes":[],"edges":[]}\n' "$*"
    ;;
  *) printf 'unknown verb\n' >&2; exit 2 ;;
esac
"#;

/// A project directory, a `PATH` directory holding only the stub `ess`, and the call log.
struct Fixture {
    _base: TempDir,
    project: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    /// `roots` each get a `system.yaml` (or an `ess-inputs.yaml` when the name ends in `inputs`)
    /// under the project; `outside` is a directory beside the project holding a `system.yaml`.
    fn new(roots: &[&str]) -> Fixture {
        let base = tempfile::tempdir().unwrap();
        let project = base.path().join("project");
        let bin = base.path().join("bin");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(base.path().join("outside")).unwrap();
        fs::write(base.path().join("outside/system.yaml"), "format: ess/20\n").unwrap();
        for root in roots {
            let dir = project.join(root);
            fs::create_dir_all(&dir).unwrap();
            let marker = if root.ends_with("inputs") {
                "ess-inputs.yaml"
            } else {
                "system.yaml"
            };
            fs::write(dir.join(marker), "format: ess/20\n").unwrap();
        }
        fs::create_dir_all(project.join("docs")).unwrap();
        Fixture {
            _base: base,
            project,
            bin,
        }
    }

    fn with_stub(self) -> Fixture {
        let path = self.bin.join("ess");
        fs::write(&path, format!("#!/bin/sh\n{STUB}")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        wait_executable(&path);
        let _ = fs::remove_file(self.log_path());
        self
    }

    fn with_stub_body(self, body: &str) -> Fixture {
        let path = self.bin.join("ess");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        wait_executable(&path);
        self
    }

    fn log_path(&self) -> PathBuf {
        self.bin.join("calls.log")
    }

    /// Every `ess specify …` invocation so far, one argument line each.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.log_path())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn env(&self) -> Env {
        Env::with_path(&self.project, &self.bin)
    }

    /// The server router with the project layered over it, as `repoview open` does.
    fn app(&self) -> Router {
        router(state()).layer(Extension(self.env()))
    }
}

/// No ETXTBSY from a concurrently forked test thread still holding the write descriptor.
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

#[tokio::test]
async fn roots_lists_every_detected_root_with_its_validate_result() {
    let fixture = Fixture::new(&["ess", "bad", "crates/a/inputs"]).with_stub();
    let (status, body) = get(fixture.app(), "/api/spec/roots").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!([
            {
                "root": "bad",
                "ok": false,
                "validate": {
                    "compiled": false,
                    "files_read": 1,
                    "problems": ["system.yaml: refused for test"],
                    "diagnostics": []
                }
            },
            {
                "root": "crates/a/inputs",
                "ok": true,
                "validate": {
                    "valid": true,
                    "system": "sys-crates/a/inputs",
                    "version": "v1",
                    "files_read": 2
                }
            },
            {
                "root": "ess",
                "ok": true,
                "validate": { "valid": true, "system": "sys-ess", "version": "v1", "files_read": 2 }
            }
        ])
    );
    let mut calls = fixture.calls();
    calls.sort();
    assert_eq!(
        calls,
        [
            "specify validate --path=bad --format json",
            "specify validate --path=crates/a/inputs --format json",
            "specify validate --path=ess --format json",
        ]
    );
}

#[tokio::test]
async fn a_validate_that_prints_no_json_is_reported_as_the_tool_failure() {
    let fixture = Fixture::new(&["ess"])
        .with_stub_body("[ \"$1\" = --version ] && exit 0\necho nope >&2\nexit 9");
    let (status, body) = get(fixture.app(), "/api/spec/roots").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!([{
            "root": "ess",
            "ok": false,
            "validate": { "tool": "ess", "exit": 9, "stderr": "nope\n" }
        }])
    );
}

#[tokio::test]
async fn ir_and_graph_pass_the_tool_output_through() {
    let fixture = Fixture::new(&["ess"]).with_stub();
    let (status, body) = get(fixture.app(), "/api/spec/roots/ess/ir").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "system": "sys",
            "argv": "specify compile --path=ess --format json",
            "entities": {}
        })
    );
    let (status, body) = get(fixture.app(), "/api/spec/roots/ess/graph").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "system": "sys",
            "argv": "specify graph --path=ess --format json",
            "groups": [],
            "nodes": [],
            "edges": []
        })
    );
}

#[tokio::test]
async fn mermaid_wraps_the_tool_text() {
    let fixture = Fixture::new(&["ess"]).with_stub();
    let (status, body) = get(fixture.app(), "/api/spec/roots/ess/mermaid").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "mermaid": "flowchart TB\n    a --> b\n" }));
    assert_eq!(
        fixture.calls(),
        ["specify graph --path=ess --format mermaid"]
    );
}

#[tokio::test]
async fn a_root_with_slashes_is_accepted_literal_and_url_encoded() {
    let fixture = Fixture::new(&["crates/a/inputs"]).with_stub();
    for uri in [
        "/api/spec/roots/crates/a/inputs/ir",
        "/api/spec/roots/crates%2Fa%2Finputs/ir",
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        assert_eq!(
            body["argv"], "specify compile --path=crates/a/inputs --format json",
            "{uri}"
        );
    }
}

#[tokio::test]
async fn the_project_root_is_the_key_tilde_and_never_a_dot_segment() {
    // Coordinator addition: a specification at the repository root is the root `.`; on the wire
    // it is `~`, because a browser drops `.` and `%2e` segments before sending.
    let fixture = Fixture::new(&[".", "ess"]).with_stub();
    let (status, body) = get(fixture.app(), "/api/spec/roots").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body[0]["root"], ".");
    for (uri, path) in [
        (
            "/api/spec/roots/~/ir",
            "specify compile --path=. --format json",
        ),
        (
            "/api/spec/roots/%7E/graph",
            "specify graph --path=. --format json",
        ),
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        assert_eq!(body["argv"], path, "{uri}");
    }
    let (status, body) = get(fixture.app(), "/api/spec/roots/~/mermaid").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        fixture.calls().last().map(String::as_str),
        Some("specify graph --path=. --format mermaid")
    );
    // `~` is the only spelling: a literal or encoded `.` key and a bare view are not.
    let before = fixture.calls().len();
    for uri in [
        "/api/spec/roots/./ir",
        "/api/spec/roots/%2E/ir",
        "/api/spec/roots/ir",
        "/api/spec/roots/~/ess/ir",
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
    }
    assert_eq!(fixture.calls().len(), before);
}

#[tokio::test]
async fn a_root_that_is_not_detected_exactly_is_404_and_runs_no_ess() {
    let fixture = Fixture::new(&["ess", "crates/a/inputs"]).with_stub();
    let outside = fixture.project.parent().unwrap().join("outside");
    let absolute = outside.to_str().unwrap().to_owned();
    let encoded_absolute = absolute.replace('/', "%2F");
    let refused = [
        "/api/spec/roots/../outside/ir".to_owned(),
        "/api/spec/roots/..%2Foutside/ir".to_owned(),
        "/api/spec/roots/%2E%2E%2Foutside/graph".to_owned(),
        format!("/api/spec/roots/{absolute}/ir"),
        format!("/api/spec/roots/{encoded_absolute}/mermaid"),
        "/api/spec/roots/docs/ir".to_owned(),
        "/api/spec/roots/crates/a/ir".to_owned(),
        "/api/spec/roots/ess/../ess/ir".to_owned(),
        "/api/spec/roots/ess%2F/ir".to_owned(),
        "/api/spec/roots/./ess/ir".to_owned(),
        // The project root is not a detected root here.
        "/api/spec/roots/ir".to_owned(),
    ];
    for uri in &refused {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
    }
    assert_eq!(fixture.calls(), Vec::<String>::new());
}

#[tokio::test]
async fn an_unknown_view_of_a_detected_root_is_404() {
    let fixture = Fixture::new(&["ess"]).with_stub();
    for uri in [
        "/api/spec/roots/ess/nope",
        "/api/spec/roots/ess",
        "/api/spec/roots/ess/ir/extra",
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
    }
    assert_eq!(fixture.calls(), Vec::<String>::new());
}

#[tokio::test]
async fn a_failing_tool_is_502_with_its_exit_and_stderr() {
    let fixture = Fixture::new(&["broken"]).with_stub();
    let (status, body) = get(fixture.app(), "/api/spec/roots/broken/ir").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body["tool"], "ess");
    assert_eq!(body["exit"], 4);
    assert_eq!(body["stderr"], "boom\n");
}

#[tokio::test]
async fn output_that_is_not_json_is_502() {
    let fixture = Fixture::new(&["ess"])
        .with_stub_body("[ \"$1\" = --version ] && { echo 'ess 7.7.7'; exit 0; }\necho 'not json'");
    for uri in ["/api/spec/roots/ess/ir", "/api/spec/roots/ess/graph"] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY, "{uri}: {body}");
        assert_eq!(body["tool"], "ess", "{uri}");
        assert_eq!(body["exit"], 0, "{uri}");
    }
}

#[tokio::test]
async fn a_missing_tool_is_503() {
    let fixture = Fixture::new(&["ess"]);
    for uri in [
        "/api/spec/roots",
        "/api/spec/roots/ess/ir",
        "/api/spec/roots/ess/graph",
        "/api/spec/roots/ess/mermaid",
    ] {
        let (status, body) = get(fixture.app(), uri).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}: {body}");
        assert_eq!(body["tool"], "ess", "{uri}");
        assert_eq!(body["exit"], Value::Null, "{uri}");
        assert!(
            body["stderr"]
                .as_str()
                .is_some_and(|text| text.contains("not found")),
            "{uri}: {body}"
        );
    }
}

#[tokio::test]
async fn the_spec_routes_sit_behind_the_token_and_host_guard() {
    let fixture = Fixture::new(&["ess"]).with_stub();
    for path in ["/api/spec/roots", "/api/spec/roots/ess/ir"] {
        let request = Request::get(path)
            .header(header::HOST, HOST)
            .body(Body::empty())
            .unwrap();
        let response = fixture.app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");

        let request = Request::get(path)
            .header(header::HOST, "evil.example:7480")
            .header(TOKEN_HEADER, token())
            .body(Body::empty())
            .unwrap();
        let response = fixture.app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }
    assert_eq!(fixture.calls(), Vec::<String>::new());
}
