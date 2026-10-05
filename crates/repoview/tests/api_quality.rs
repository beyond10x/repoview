//! story:quality-page: language detection, `codegate capabilities`, background assessments and
//! `GET /api/quality`, against real directories and a stub `codegate`.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::{Extension, Router};
use common::{bin_dir, git};
use repoview::api::quality::{
    ASSESS_TIMEOUT, Assessments, TOOL, detect_languages, parse_capabilities,
};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const PORT: u16 = 7480;
const HOST: &str = "127.0.0.1:7480";

/// What the stub's `assess` prints for markdown: an assessment document as codegate shapes it.
const ASSESSMENT: &str = r#"{"rating":"B-","score_max":100,"scores":{"overall":67,"maintainability":67},"finding_counts":{"markdown_missing_h1":2},"top_findings":[{"kind":"markdown_missing_h1","severity":"warning","reason":"Document has no H1 title.","location":{"uri":"docs/a.md"}}]}"#;

/// The capabilities the stub reports unless a test says otherwise: go and markdown, as the
/// installed codegate does.
const CAPABILITIES: &str = r#"[{"language":"go","name":"goast","capabilities":[]},{"language":"markdown","name":"markdown","capabilities":[]}]"#;

/// A stub `codegate` in a fresh `PATH` directory that also holds the real `git` and `sleep`.
///
/// `capabilities` prints `capabilities`; `assess` for markdown sleeps `markdown_delay` seconds and
/// prints [`ASSESSMENT`]; for go it fails with `go: build failed`; for anything else it prints
/// codegate's own "not wired" error. Every call's argv is appended to `calls.log` in the dir.
struct Stub {
    dir: TempDir,
}

impl Stub {
    fn new(capabilities: &str, markdown_delay: &str) -> Stub {
        let dir = bin_dir(&["git", "sleep"]);
        let log = dir.path().join("calls.log");
        let body = format!(
            r#"printf '%s\n' "$*" >> '{log}'
if [ "$1" = capabilities ]; then printf '%s' '{capabilities}'; exit 0; fi
case "$4" in
  markdown) sleep {markdown_delay}; printf '%s' '{ASSESSMENT}' ;;
  go) printf 'go: build failed\n' >&2; exit 2 ;;
  *) printf 'language "%s" is not wired\n' "$4" >&2; exit 1 ;;
esac"#,
            log = log.display(),
        );
        write_stub(dir.path(), "codegate", &body);
        // The readiness probe in `write_stub` ran the stub once; that call is not the test's.
        fs::remove_file(&log).ok();
        Stub { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn codegate(&self) -> PathBuf {
        self.dir.path().join("codegate")
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("calls.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// Write an executable `/bin/sh` stub and wait until it can be executed (no ETXTBSY from a
/// concurrently forked test thread still holding the write descriptor).
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

/// A Git repository with `files` written and `tracked` of them added to the index.
fn project(files: &[&str], tracked: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    for file in files {
        let path = dir.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x\n").unwrap();
    }
    if !tracked.is_empty() {
        let mut args = vec!["add", "--"];
        args.extend_from_slice(tracked);
        git(dir.path(), &args);
    }
    dir
}

fn languages(root: &Path) -> Vec<String> {
    let path = bin_dir(&["git"]);
    detect_languages(&Env::with_path(root, path.path()))
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// `document()` once no language is `running` any more.
fn settled(assessments: &Arc<Assessments>) -> Value {
    let started = Instant::now();
    loop {
        let document = assessments.document();
        let running = document["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|language| language["status"] == "running");
        if !running {
            return document;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "still running: {document}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn entry<'a>(document: &'a Value, language: &str) -> &'a Value {
    document["languages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["language"] == language)
        .unwrap_or_else(|| panic!("no {language} entry in {document}"))
}

fn language_names(document: &Value) -> Vec<&str> {
    document["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["language"].as_str().unwrap())
        .collect()
}

// Acceptance 1: detection from the project root.

#[test]
fn each_marker_file_detects_its_language() {
    for (file, language) in [
        ("go.mod", "go"),
        ("Cargo.toml", "rust"),
        ("package.json", "typescript"),
        ("pom.xml", "java"),
        ("build.gradle", "java"),
        ("build.gradle.kts", "java"),
    ] {
        let dir = project(&[file], &[]);
        assert_eq!(languages(dir.path()), vec![language.to_owned()], "{file}");
    }
}

#[test]
fn a_tracked_markdown_file_anywhere_detects_markdown() {
    let dir = project(&["docs/deep/notes.md"], &["docs/deep/notes.md"]);
    assert_eq!(languages(dir.path()), vec!["markdown".to_owned()]);
}

#[test]
fn an_untracked_markdown_file_does_not_detect_markdown() {
    let dir = project(&["README.md", "go.mod"], &["go.mod"]);
    assert_eq!(languages(dir.path()), vec!["go".to_owned()]);
}

#[test]
fn every_language_is_detected_once_in_a_fixed_order() {
    let dir = project(
        &[
            "README.md",
            "pom.xml",
            "build.gradle",
            "package.json",
            "Cargo.toml",
            "go.mod",
        ],
        &["README.md"],
    );
    assert_eq!(
        languages(dir.path()),
        ["go", "rust", "typescript", "java", "markdown"].map(str::to_owned)
    );
}

#[test]
fn an_empty_directory_has_no_languages() {
    let dir = tempfile::tempdir().unwrap();
    assert!(languages(dir.path()).is_empty());
}

// Acceptance 1 and 4: capabilities parsing.

#[test]
fn capabilities_are_the_language_field_of_each_entry() {
    assert_eq!(
        parse_capabilities(CAPABILITIES).unwrap(),
        vec!["go".to_owned(), "markdown".to_owned()]
    );
}

#[test]
fn capabilities_that_are_not_a_json_array_are_an_error() {
    for text in ["", "not json", r#"{"language":"go"}"#] {
        let error = parse_capabilities(text).unwrap_err();
        assert!(error.contains("codegate capabilities"), "{text:?}: {error}");
    }
}

#[test]
fn a_capabilities_entry_without_a_language_is_skipped() {
    assert_eq!(
        parse_capabilities(r#"[{"name":"x"},{"language":"go"},{"language":7}]"#).unwrap(),
        vec!["go".to_owned()]
    );
}

// Acceptance 2 and 4: background assessments with a stub codegate.

#[test]
fn the_assess_timeout_is_120_seconds() {
    assert_eq!(ASSESS_TIMEOUT, Duration::from_secs(120));
    assert_eq!(TOOL, "codegate");
}

#[test]
fn a_supported_language_is_assessed_and_its_document_passed_through() {
    let stub = Stub::new(CAPABILITIES, "0");
    let dir = project(&["README.md"], &["README.md"]);
    let env = Env::with_path(dir.path(), stub.path());
    let document = settled(&Assessments::start(env, stub.codegate(), ASSESS_TIMEOUT));
    assert_eq!(document["tool"], "codegate");
    assert_eq!(
        document["tool_path"],
        json!(stub.codegate().to_str().unwrap())
    );
    let markdown = entry(&document, "markdown");
    assert_eq!(markdown["status"], "assessed");
    assert_eq!(markdown["reason"], Value::Null);
    let expected: Value = serde_json::from_str(ASSESSMENT).unwrap();
    assert_eq!(markdown["assessment"], expected);
    let root = dir.path().to_str().unwrap();
    assert!(
        stub.calls().contains(&format!(
            "--root {root} --language markdown --format json assess --gate all"
        )),
        "{:?}",
        stub.calls()
    );
}

#[test]
fn an_unsupported_language_is_not_assessed_with_the_reason_and_never_run() {
    let stub = Stub::new(CAPABILITIES, "0");
    let dir = project(&["Cargo.toml", "package.json"], &[]);
    let env = Env::with_path(dir.path(), stub.path());
    let document = settled(&Assessments::start(env, stub.codegate(), ASSESS_TIMEOUT));
    for language in ["rust", "typescript"] {
        let entry = entry(&document, language);
        assert_eq!(entry["status"], "not-assessed", "{language}");
        assert_eq!(
            entry["reason"],
            json!(format!("codegate does not support {language}"))
        );
        assert_eq!(entry["assessment"], Value::Null, "{language}");
    }
    assert_eq!(stub.calls(), vec!["capabilities".to_owned()]);
}

#[test]
fn supported_languages_come_from_capabilities_not_from_a_list() {
    let stub = Stub::new(r#"[{"language":"rust"}]"#, "0");
    let dir = project(&["Cargo.toml", "go.mod"], &[]);
    let env = Env::with_path(dir.path(), stub.path());
    let document = settled(&Assessments::start(env, stub.codegate(), ASSESS_TIMEOUT));
    // rust is wired in this stub's capabilities, so it is assessed (the stub then fails it).
    assert_eq!(entry(&document, "rust")["status"], "failed");
    assert_eq!(entry(&document, "go")["status"], "not-assessed");
    assert_eq!(
        entry(&document, "go")["reason"],
        "codegate does not support go"
    );
}

#[test]
fn a_failing_assess_is_failed_with_its_stderr_and_no_assessment() {
    let stub = Stub::new(CAPABILITIES, "0");
    let dir = project(&["go.mod"], &[]);
    let env = Env::with_path(dir.path(), stub.path());
    let document = settled(&Assessments::start(env, stub.codegate(), ASSESS_TIMEOUT));
    let go = entry(&document, "go");
    assert_eq!(go["status"], "failed");
    assert_eq!(go["reason"], "codegate assess failed");
    assert!(
        go["stderr"].as_str().unwrap().contains("go: build failed"),
        "{go}"
    );
    assert_eq!(go["assessment"], Value::Null);
}

#[test]
fn an_assess_past_the_timeout_is_failed_and_says_so() {
    let stub = Stub::new(CAPABILITIES, "30");
    let dir = project(&["README.md"], &["README.md"]);
    let env = Env::with_path(dir.path(), stub.path());
    let started = Instant::now();
    let document = settled(&Assessments::start(
        env,
        stub.codegate(),
        Duration::from_millis(300),
    ));
    assert!(started.elapsed() < Duration::from_secs(10));
    let markdown = entry(&document, "markdown");
    assert_eq!(markdown["status"], "failed");
    assert_eq!(
        markdown["reason"], "codegate assess timed out after 0.3 s",
        "{markdown}"
    );
    assert!(
        markdown["stderr"].as_str().unwrap().contains("timed out"),
        "{markdown}"
    );
    assert_eq!(markdown["assessment"], Value::Null);
}

#[test]
fn an_assessment_is_running_until_it_finishes() {
    let stub = Stub::new(CAPABILITIES, "2");
    let dir = project(&["README.md", "Cargo.toml"], &["README.md"]);
    let env = Env::with_path(dir.path(), stub.path());
    let assessments = Assessments::start(env, stub.codegate(), ASSESS_TIMEOUT);
    let first = assessments.document();
    assert_eq!(entry(&first, "markdown")["status"], "running");
    assert_eq!(entry(&first, "markdown")["assessment"], Value::Null);
    assert_eq!(entry(&first, "rust")["status"], "not-assessed");
    assert_eq!(
        entry(&settled(&assessments), "markdown")["status"],
        "assessed"
    );
}

#[test]
fn a_failing_capabilities_call_fails_every_language_with_its_stderr() {
    let dir = project(&["go.mod", "README.md"], &["README.md"]);
    let bin = bin_dir(&["git"]);
    let codegate = write_stub(
        bin.path(),
        "codegate",
        "printf 'capabilities broke\\n' >&2; exit 3",
    );
    let env = Env::with_path(dir.path(), bin.path());
    let document = settled(&Assessments::start(env, codegate, ASSESS_TIMEOUT));
    assert_eq!(language_names(&document), vec!["go", "markdown"]);
    for language in ["go", "markdown"] {
        let entry = entry(&document, language);
        assert_eq!(entry["status"], "failed", "{language}");
        assert_eq!(entry["reason"], "codegate capabilities failed");
        assert!(
            entry["stderr"]
                .as_str()
                .unwrap()
                .contains("capabilities broke")
        );
        assert_eq!(entry["assessment"], Value::Null);
    }
}

#[test]
fn an_assess_that_prints_no_json_is_failed() {
    let dir = project(&["README.md"], &["README.md"]);
    let bin = bin_dir(&["git"]);
    let codegate = write_stub(
        bin.path(),
        "codegate",
        &format!(
            "if [ \"$1\" = capabilities ]; then printf '%s' '{CAPABILITIES}'; exit 0; fi\n\
             printf 'not json'"
        ),
    );
    let env = Env::with_path(dir.path(), bin.path());
    let document = settled(&Assessments::start(env, codegate, ASSESS_TIMEOUT));
    let markdown = entry(&document, "markdown");
    assert_eq!(markdown["status"], "failed");
    assert_eq!(
        markdown["reason"],
        "codegate assess printed no JSON document"
    );
    assert_eq!(markdown["assessment"], Value::Null);
}

// Acceptance 2: the HTTP surface, through the router with the project handed in as
// `Extension<Env>`, as `repoview open` layers it.

fn app(root: &Path, path: &Path) -> (Router, String) {
    let token = new_token();
    let state = AppState {
        token: token.clone(),
        port: PORT,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    };
    let app = router(state).layer(Extension(Env::with_path(root, path)));
    (app, token)
}

async fn get_quality(app: &Router, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::get("/api/quality").header(header::HOST, HOST);
    if let Some(token) = token {
        request = request.header(TOKEN_HEADER, token);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, value)
}

#[tokio::test(flavor = "multi_thread")]
async fn without_codegate_on_path_the_api_is_503_naming_the_tool() {
    let dir = project(&["go.mod"], &[]);
    let bin = bin_dir(&["git"]);
    let (app, token) = app(dir.path(), bin.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        json!({ "tool": "codegate", "exit": null, "stderr": "codegate not found on PATH" })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_api_refuses_a_request_without_the_token() {
    let stub = Stub::new(CAPABILITIES, "0");
    let dir = project(&["go.mod"], &[]);
    let (app, _) = app(dir.path(), stub.path());
    let (status, _) = get_quality(&app, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(stub.calls().is_empty(), "{:?}", stub.calls());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_api_answers_running_then_every_language_with_its_status() {
    let stub = Stub::new(CAPABILITIES, "2");
    let dir = project(&["README.md", "go.mod", "Cargo.toml"], &["README.md"]);
    let (app, token) = app(dir.path(), stub.path());
    let (status, first) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["tool"], "codegate");
    assert_eq!(first["tool_path"], json!(stub.codegate().to_str().unwrap()));
    assert_eq!(language_names(&first), vec!["go", "rust", "markdown"]);
    assert_eq!(entry(&first, "markdown")["status"], "running");
    let started = Instant::now();
    let done = loop {
        let (status, document) = get_quality(&app, Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        if entry(&document, "markdown")["status"] != "running" {
            break document;
        }
        assert!(started.elapsed() < Duration::from_secs(20), "{document}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(entry(&done, "markdown")["status"], "assessed");
    assert_eq!(
        entry(&done, "markdown")["assessment"],
        serde_json::from_str::<Value>(ASSESSMENT).unwrap()
    );
    assert_eq!(entry(&done, "go")["status"], "failed");
    assert_eq!(entry(&done, "rust")["status"], "not-assessed");
    // One capabilities call and one assess per supported language, however often it is polled.
    let calls = stub.calls();
    assert_eq!(
        calls.iter().filter(|call| *call == "capabilities").count(),
        1
    );
    assert_eq!(calls.len(), 3, "{calls:?}");
}

/// Round 1: a first request dropped while the run is starting (here: inside a slow
/// `codegate capabilities`) must not let a second request start a second run.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_dropped_while_the_run_starts_leaves_exactly_one_run() {
    let bin = bin_dir(&["git", "sleep"]);
    let log = bin.path().join("calls.log");
    write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
printf '%s\n' "$1 $4" >> '{log}'
if [ "$1" = capabilities ]; then sleep 2; printf '%s' '{CAPABILITIES}'; exit 0; fi
printf '%s' '{ASSESSMENT}'"#,
            log = log.display()
        ),
    );
    let dir = project(&["README.md"], &["README.md"]);
    let (app, token) = app(dir.path(), bin.path());
    let first = {
        let (app, token) = (app.clone(), token.clone());
        tokio::spawn(async move { get_quality(&app, Some(&token)).await })
    };
    let started = Instant::now();
    while !fs::read_to_string(&log).is_ok_and(|text| text.contains("capabilities")) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "no capabilities call"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    first.abort();
    let _ = first.await;
    let (status, _) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    tokio::time::sleep(Duration::from_secs(3)).await;
    let calls = fs::read_to_string(&log).unwrap();
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("capabilities"))
            .count(),
        1,
        "{calls}"
    );
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.ends_with("markdown"))
            .count(),
        1,
        "{calls}"
    );
}
