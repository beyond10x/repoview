//! story:plan-pages acceptance 1: `/api/plan/*` runs `aep` in the project root and passes its JSON
//! through, against a stub `aep` on a test `PATH`.

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

/// A project directory, a `PATH` directory and the file the stub `aep` records its run in.
struct Fixture {
    project: TempDir,
    bin: TempDir,
    record: PathBuf,
}

impl Fixture {
    /// A stub `aep` that records its working directory and argv, then runs `body`.
    fn with_aep(body: &str) -> Fixture {
        let project = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let record = bin.path().join("record");
        let script = format!(
            "[ \"$1\" = --probe ] && exit 0\nprintf '%s\\n' \"$PWD\" \"$@\" > '{}'\n{body}",
            record.display()
        );
        stub(bin.path(), "aep", &script);
        Fixture {
            project,
            bin,
            record,
        }
    }

    /// No `aep` anywhere on the `PATH`.
    fn without_aep() -> Fixture {
        let project = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let record = bin.path().join("record");
        Fixture {
            project,
            bin,
            record,
        }
    }

    fn env(&self) -> Env {
        Env::with_path(self.project.path(), self.bin.path().as_os_str())
    }

    /// The stub's working directory and arguments, or `None` when it never ran.
    fn recorded(&self) -> Option<Vec<String>> {
        fs::read_to_string(&self.record)
            .ok()
            .map(|text| text.lines().map(str::to_owned).collect())
    }

    async fn get(&self, uri: &str) -> (StatusCode, String, String) {
        self.get_with(uri, &[(TOKEN_HEADER, &token())]).await
    }

    async fn get_with(&self, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, String, String) {
        let mut request = Request::get(uri).header(header::HOST, HOST);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = router(state())
            .layer(Extension(self.env()))
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_owned())
            .unwrap_or_default();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }
}

/// Write an executable `/bin/sh` stub and wait until it can be executed (no ETXTBSY from a
/// concurrently forked test thread still holding the write descriptor).
fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("--probe").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return path,
        }
    }
    panic!("stub {name} stayed busy");
}

fn canonical(path: &Path) -> String {
    fs::canonicalize(path).unwrap().to_str().unwrap().to_owned()
}

/// Deliberately irregular spacing, so a re-serialisation would show.
const DOCUMENT: &str = "{\"columns\" :[ 1,2 ],  \"zz\": \"keep\"}";

fn json_of(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|error| panic!("not JSON ({error}): {body}"))
}

#[tokio::test]
async fn every_route_runs_its_aep_command_in_the_project_root_and_passes_the_json_through() {
    let id = "story:plan-pages";
    let routes: [(String, Vec<&str>); 7] = [
        (
            "/api/plan/board".into(),
            vec!["plan", "artifact", "board", "--format", "json"],
        ),
        (
            "/api/plan/artifacts".into(),
            vec!["plan", "artifact", "list", "--format", "json"],
        ),
        (
            "/api/plan/graph".into(),
            vec!["plan", "artifact", "graph", "--format", "json"],
        ),
        (
            "/api/plan/validate".into(),
            vec!["plan", "artifact", "validate", "--format", "json"],
        ),
        (
            format!("/api/plan/artifacts/{id}"),
            vec!["plan", "artifact", "show", "--format", "json", "--", id],
        ),
        (
            format!("/api/plan/artifacts/{id}/history"),
            vec!["plan", "artifact", "history", "--format", "json", "--", id],
        ),
        (
            format!("/api/plan/artifacts/{id}/explain"),
            vec!["plan", "artifact", "explain", "--format", "json", "--", id],
        ),
    ];
    for (uri, argv) in routes {
        let fixture = Fixture::with_aep(&format!("printf '%s' '{DOCUMENT}'"));
        let (status, content_type, body) = fixture.get(&uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        assert!(
            content_type.starts_with("application/json"),
            "{uri}: {content_type}"
        );
        assert_eq!(
            body, DOCUMENT,
            "{uri}: the body is aep's stdout, byte for byte"
        );
        let mut expected = vec![canonical(fixture.project.path())];
        expected.extend(argv.iter().map(|arg| (*arg).to_owned()));
        assert_eq!(fixture.recorded(), Some(expected), "{uri}");
    }
}

#[tokio::test]
async fn a_plan_route_without_the_token_is_403_and_starts_no_process() {
    let fixture = Fixture::with_aep(&format!("printf '%s' '{DOCUMENT}'"));
    for uri in ["/api/plan/board", "/api/plan/artifacts/story:x"] {
        let (status, _, _) = fixture.get_with(uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        let wrong = "b".repeat(64);
        let (status, _, _) = fixture.get_with(uri, &[(TOKEN_HEADER, &wrong)]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
    }
    assert_eq!(fixture.recorded(), None);
}

#[tokio::test]
async fn an_id_outside_the_pattern_is_400_and_starts_no_process() {
    let bad = [
        "story;rm:x",      // `;`
        "story:x;y",       // `;` in the name
        "..",              // dot-dot
        "%2e%2e",          // dot-dot, encoded
        "story:..",        // dot-dot as the name
        "story:a%20b",     // a space
        "story%20x:y",     // a space in the namespace
        "story",           // no colon
        "story:",          // empty name
        ":x",              // empty namespace
        "a:b:c",           // two colons
        "story:a%2Fb",     // an encoded slash
        "story:a%00",      // NUL
        "st%C3%B6ry:x",    // non-ASCII
        "story:.x",        // a part that starts with a dot
        ".hidden:x",       // a namespace that starts with a dot
        "..:x",            // dot-dot as the namespace
        "story:a%2B",      // `+`
        "story:a%2Ab",     // `*`
        "story%3Ax%0Ay:z", // newline
    ];
    for suffix in ["", "/history", "/explain"] {
        for id in bad {
            let fixture = Fixture::with_aep(&format!("printf '%s' '{DOCUMENT}'"));
            let uri = format!("/api/plan/artifacts/{id}{suffix}");
            let (status, content_type, body) = fixture.get(&uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
            assert!(
                content_type.starts_with("application/json"),
                "{uri}: {content_type}"
            );
            assert!(json_of(&body)["error"].is_string(), "{uri}: {body}");
            assert_eq!(fixture.recorded(), None, "{uri}: aep must not run");
        }
    }
}

#[tokio::test]
async fn the_patterns_whole_alphabet_is_accepted() {
    let fixture = Fixture::with_aep(&format!("printf '%s' '{DOCUMENT}'"));
    // `^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$`: aep's grammar without `/`.
    for id in [
        "abc-xyz-0189:z-9-a",
        "ABC_xyz:Z_9.a-b",
        "vision:O2",
        "Story:x",
        "_a:-b",
        "a..b:c.d.",
    ] {
        let (status, _, body) = fixture.get(&format!("/api/plan/artifacts/{id}")).await;
        assert_eq!(status, StatusCode::OK, "{id}: {body}");
        assert_eq!(fixture.recorded().unwrap().last().unwrap(), id);
    }
}

#[tokio::test]
async fn a_non_zero_aep_exit_is_502_with_tool_exit_and_stderr() {
    let fixture = Fixture::with_aep("echo 'error: no such artifact' >&2\nexit 3");
    for uri in ["/api/plan/board", "/api/plan/artifacts/story:x"] {
        let (status, content_type, body) = fixture.get(uri).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY, "{uri}: {body}");
        assert!(content_type.starts_with("application/json"), "{uri}");
        let value = json_of(&body);
        assert_eq!(value["tool"], "aep", "{uri}");
        assert_eq!(value["exit"], 3, "{uri}");
        assert_eq!(value["stderr"], "error: no such artifact\n", "{uri}");
    }
}

#[tokio::test]
async fn a_failing_validate_keeps_the_problems_aep_printed() {
    let problems = "{\"artifacts\": 2, \"problems\": [\"story/a.md: declares kind storyy\"]}";
    let fixture = Fixture::with_aep(&format!(
        "printf '%s' '{problems}'\necho 'warning: 1 document unread' >&2\nexit 1"
    ));
    let (status, _, body) = fixture.get("/api/plan/validate").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    let value = json_of(&body);
    assert_eq!(value["tool"], "aep");
    assert_eq!(value["exit"], 1);
    assert_eq!(value["stderr"], "warning: 1 document unread\n");
    assert_eq!(value["stdout"], problems);
}

#[tokio::test]
async fn a_long_stderr_is_cut_to_the_diagnostic_limit() {
    let fixture = Fixture::with_aep(
        // Builtins only: the test PATH holds nothing but the stub.
        "i=0\nwhile [ $i -lt 2000 ]; do printf eeeeeeeeee >&2; i=$((i+1)); done\nexit 1",
    );
    let (status, _, body) = fixture.get("/api/plan/board").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let stderr = json_of(&body)["stderr"].as_str().unwrap().to_owned();
    assert_eq!(stderr.len(), repoview_sources::DIAGNOSTIC_LIMIT);
}

#[tokio::test]
async fn no_aep_on_path_is_503_with_the_same_shape() {
    let fixture = Fixture::without_aep();
    for uri in ["/api/plan/board", "/api/plan/artifacts/story:x/explain"] {
        let (status, content_type, body) = fixture.get(uri).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}: {body}");
        assert!(content_type.starts_with("application/json"), "{uri}");
        let value = json_of(&body);
        assert_eq!(value["tool"], "aep", "{uri}");
        assert_eq!(value["exit"], Value::Null, "{uri}");
        assert!(
            value["stderr"]
                .as_str()
                .unwrap()
                .contains("aep not found on PATH"),
            "{uri}: {body}"
        );
    }
}

#[tokio::test]
async fn an_exit_zero_that_printed_no_json_is_502() {
    let fixture = Fixture::with_aep("echo 'not json'");
    let (status, _, body) = fixture.get("/api/plan/artifacts").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    let value = json_of(&body);
    assert_eq!(value["tool"], "aep");
    assert_eq!(value["exit"], 0);
    assert!(value["stderr"].as_str().unwrap().contains("JSON"), "{body}");
}

#[tokio::test]
async fn a_path_below_an_artifact_route_is_the_api_404() {
    let fixture = Fixture::with_aep(&format!("printf '%s' '{DOCUMENT}'"));
    let (status, _, _) = fixture.get("/api/plan/artifacts/story:x/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(fixture.recorded(), None);
}
