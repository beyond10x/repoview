//! story:quality-codegate: `GET /api/quality` names the beyond10x `codegate` it found, its version
//! and the commands its `--help` lists, and says why there is no assessment. The background-run
//! machinery kept from story:quality-page is exercised directly with a stub program.

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
use common::bin_dir;
use repoview::api::quality::{ASSESS_TIMEOUT, BackgroundRun, TOOL, parse_commands, reason};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use repoview_sources::{Env, read_all};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const PORT: u16 = 7480;
const HOST: &str = "127.0.0.1:7480";

/// `codegate --help` of the beyond10x codegate 0.3.0, verbatim (the `evaluate` line ends in two
/// spaces).
const HELP_0_3_0: &str = "Evaluate normalized dependency facts against a language-neutral policy

Usage: codegate <COMMAND>

Commands:
  evaluate
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
";

const REASON_0_3_0: &str =
    "codegate 0.3.0 evaluates supplied dependency facts only; it has no source assessment yet";

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

/// A `codegate` stub in its own `PATH` directory that appends every call's argv to `calls.log`.
struct Stub {
    dir: TempDir,
}

impl Stub {
    /// The beyond10x shape: `--version` prints `codegate <version>`, `--help` prints `help`
    /// after `help_delay` seconds, anything else fails.
    fn rust(version: &str, help: &str, help_delay: &str) -> Stub {
        Stub::with_body(&format!(
            r#"case "$1" in
  --version) printf 'codegate {version}\n' ;;
  --help) sleep {help_delay}; printf '%s' '{help}' ;;
  *) printf 'unexpected call: %s\n' "$*" >&2; exit 64 ;;
esac"#
        ))
    }

    /// The Go shape: every flag it does not know, `--version` included, is an error with exit 1.
    fn go() -> Stub {
        Stub::with_body(r#"printf 'Error: unknown flag: %s\n' "$1" >&2; exit 1"#)
    }

    fn with_body(body: &str) -> Stub {
        let dir = bin_dir(&["sleep"]);
        let log = dir.path().join("calls.log");
        let body = format!(
            r#"[ "$1" = probe-busy ] && exit 0
printf '%s\n' "$*" >> '{log}'
{body}"#,
            log = log.display(),
        );
        write_stub(dir.path(), "codegate", &body);
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

/// `PATH` made of `stubs`' directories, in order.
fn search_path(stubs: &[&Stub]) -> std::ffi::OsString {
    std::env::join_paths(stubs.iter().map(|stub| stub.path())).unwrap()
}

fn app(root: &Path, path: impl Into<std::ffi::OsString>) -> (Router, String) {
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

fn not_found() -> Value {
    json!({ "tool": "codegate", "exit": null, "stderr": "beyond10x codegate not found on PATH" })
}

// Acceptance 2: `commands` from `codegate --help`, and the reason.

#[test]
fn the_commands_of_codegate_0_3_0_are_evaluate() {
    assert_eq!(parse_commands(HELP_0_3_0), vec!["evaluate".to_owned()]);
}

#[test]
fn commands_are_every_entry_of_the_commands_section_but_help() {
    let help = "About\n\nUsage: codegate <COMMAND>\n\nCommands:\n  evaluate  Evaluate facts\n  \
                assess    Assess a tree\n            that wraps\n  help      Print help\n\n\
                Options:\n  -h, --help  Print help\n";
    assert_eq!(
        parse_commands(help),
        vec!["evaluate".to_owned(), "assess".to_owned()]
    );
}

#[test]
fn help_without_a_commands_section_lists_no_commands() {
    assert!(parse_commands("Usage: codegate [OPTIONS]\n\nOptions:\n  -h  Print help\n").is_empty());
    assert!(parse_commands("").is_empty());
}

#[test]
fn the_reason_for_codegate_0_3_0_is_the_story_text() {
    assert_eq!(reason("0.3.0", &["evaluate".to_owned()]), REASON_0_3_0);
}

#[test]
fn a_reason_names_the_version_and_says_there_is_no_source_assessment() {
    // Correction round 1: a command beyond `evaluate` may assess; the reason must not say none does.
    assert_eq!(
        reason("0.9.1", &["evaluate".to_owned(), "lint".to_owned()]),
        "codegate 0.9.1 offers evaluate, lint; repoview does not read an assessment from it yet"
    );
    assert_eq!(
        reason("0.9.1", &[]),
        "codegate 0.9.1 offers no commands; it has no source assessment yet"
    );
}

// Acceptance 2: the HTTP surface, through the router with the project handed in as
// `Extension<Env>`, as `repoview open` layers it.

#[tokio::test(flavor = "multi_thread")]
async fn the_api_names_the_rust_codegate_its_version_commands_and_reason() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "tool": "codegate",
            "tool_path": stub.codegate().to_str().unwrap(),
            "tool_version": "0.3.0",
            "skipped": [],
            "commands": ["evaluate"],
            "assessment": null,
            "reason": REASON_0_3_0,
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_go_codegate_before_the_rust_one_is_skipped_and_named() {
    let go = Stub::go();
    let rust = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), search_path(&[&go, &rust]));
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tool_path"], json!(rust.codegate().to_str().unwrap()));
    assert_eq!(body["skipped"], json!([go.codegate().to_str().unwrap()]));
    // The Go codegate is asked for its version and nothing else.
    assert_eq!(go.calls(), vec!["--version".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_a_go_codegate_is_503_beyond10x_codegate_not_found() {
    let go = Stub::go();
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), go.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, not_found());
    assert_eq!(go.calls(), vec!["--version".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn no_codegate_on_path_is_503_beyond10x_codegate_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), empty.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, not_found());
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_come_from_help_not_from_a_list() {
    let help = "Usage: codegate <COMMAND>\n\nCommands:\n  evaluate  x\n  survey    y\n  help  z\n";
    let stub = Stub::rust("0.4.0-dev", help, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["commands"], json!(["evaluate", "survey"]));
    assert_eq!(body["tool_version"], "0.4.0-dev");
    assert_eq!(body["assessment"], Value::Null);
    assert_eq!(
        body["reason"],
        json!(reason(
            "0.4.0-dev",
            &["evaluate".to_owned(), "survey".to_owned()]
        ))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_help_is_502_with_its_stderr() {
    let stub = Stub::with_body(
        r#"case "$1" in
  --version) printf 'codegate 0.3.0\n' ;;
  *) printf 'help broke\n' >&2; exit 3 ;;
esac"#,
    );
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body["tool"], "codegate");
    assert_eq!(body["exit"], Value::Null);
    assert!(
        body["stderr"].as_str().unwrap().contains("help broke"),
        "{body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_api_refuses_a_request_without_the_token() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, _) = app(dir.path(), stub.path());
    let (status, _) = get_quality(&app, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(stub.calls().is_empty(), "{:?}", stub.calls());
}

// Acceptance 4: the API and the snapshot's `quality` source report the same binary and version.

#[tokio::test(flavor = "multi_thread")]
async fn the_api_and_the_snapshot_source_report_the_same_binary_and_version() {
    let go = Stub::go();
    let rust = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let path = search_path(&[&go, &rust]);
    let (app, token) = app(dir.path(), path.clone());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let section = read_all(&Env::with_path(dir.path(), path))
        .into_iter()
        .find(|section| section.source_id == "quality")
        .unwrap();
    assert_eq!(body["tool_path"], json!(section.tool_path));
    assert_eq!(body["tool_version"], json!(section.tool_version));
    assert_eq!(body["skipped"], section.summary["skipped"]);
}

// Acceptance 5: no assessment process starts, and the probe runs once per server run.

#[tokio::test(flavor = "multi_thread")]
async fn no_assessment_process_starts_however_often_the_page_asks() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    for _ in 0..3 {
        let (status, body) = get_quality(&app, Some(&token)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["assessment"], Value::Null);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Correction round 1: every request re-locates (`--version`); `--help` runs once while the
    // located binary stays the same, and nothing else is ever started.
    assert_eq!(probe_calls(&stub), (3, 1), "{:?}", stub.calls());
}

/// `(--version calls, --help calls)` of `stub`, asserting it was asked nothing else.
fn probe_calls(stub: &Stub) -> (usize, usize) {
    let calls = stub.calls();
    assert!(
        calls
            .iter()
            .all(|call| call == "--version" || call == "--help"),
        "{calls:?}"
    );
    let versions = calls.iter().filter(|call| *call == "--version").count();
    (versions, calls.len() - versions)
}

/// Correction round 1: a `codegate` removed from `PATH` while the server runs is not reported
/// as found any longer.
#[tokio::test(flavor = "multi_thread")]
async fn a_codegate_removed_while_the_server_runs_is_no_longer_found() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    fs::remove_file(stub.codegate()).unwrap();
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, not_found());
}

/// Correction round 1: a different binary at the same path (an in-place upgrade whose help
/// changed) is probed again, and its commands are the new ones.
#[tokio::test(flavor = "multi_thread")]
async fn an_upgraded_codegate_is_probed_again() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "0");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let (_, first) = get_quality(&app, Some(&token)).await;
    assert_eq!(first["commands"], json!(["evaluate"]));
    let help = "Commands:\n  evaluate  x\n  survey    y\n";
    let upgraded = Stub::rust("0.4.0", help, "0");
    fs::copy(upgraded.codegate(), stub.codegate()).unwrap();
    // Wait out ETXTBSY from a concurrently forked test thread still holding the write descriptor.
    for _ in 0..200 {
        match Command::new(stub.codegate()).arg("probe-busy").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => break,
        }
    }
    let (status, second) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(second["tool_version"], "0.4.0");
    assert_eq!(second["commands"], json!(["evaluate", "survey"]));
}

/// Correction round 1: ANSI styling in `--help` (clap colours it under `CLICOLOR_FORCE`) does
/// not hide a command.
#[test]
fn ansi_styling_in_help_is_ignored() {
    let help = "\u{1b}[1m\u{1b}[4mCommands:\u{1b}[0m\n  \u{1b}[1mevaluate\u{1b}[0m  \n  \
                \u{1b}[1mhelp\u{1b}[0m      Print help\n";
    assert_eq!(parse_commands(help), vec!["evaluate".to_owned()]);
}

/// Round 1 of story:quality-page, kept: a first request dropped while the run is starting (here:
/// inside a slow `codegate --help`) must not let a second request start a second probe.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_dropped_while_the_run_starts_leaves_exactly_one_run() {
    let stub = Stub::rust("0.3.0", HELP_0_3_0, "2");
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path(), stub.path());
    let first = {
        let (app, token) = (app.clone(), token.clone());
        tokio::spawn(async move { get_quality(&app, Some(&token)).await })
    };
    let started = Instant::now();
    while !stub.calls().iter().any(|call| call == "--help") {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "no --help call"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    first.abort();
    let _ = first.await;
    let (status, body) = get_quality(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["commands"], json!(["evaluate"]));
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(probe_calls(&stub), (2, 1), "{:?}", stub.calls());
}

// Acceptance 5: the background-run machinery, kept for the assessment command a later codegate
// release adds, run directly against a stub program.

/// `document()` once the run is no longer `running`.
fn settled(run: &Arc<BackgroundRun>) -> Value {
    let started = Instant::now();
    loop {
        let document = run.document();
        if document["status"] != "running" {
            return document;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "still running: {document}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn program(body: &str) -> (TempDir, PathBuf) {
    let bin = bin_dir(&["sleep"]);
    let path = write_stub(bin.path(), "assessor", body);
    (bin, path)
}

/// Start `program assess` with `bin` as both the project root and `PATH`, so the root outlives
/// the run.
fn start(bin: &TempDir, program: PathBuf, timeout: Duration) -> Arc<BackgroundRun> {
    let env = Env::with_path(bin.path(), bin.path());
    BackgroundRun::start(env, program, vec!["assess".to_owned()], timeout)
}

#[test]
fn the_assess_timeout_is_120_seconds() {
    assert_eq!(ASSESS_TIMEOUT, Duration::from_secs(120));
    assert_eq!(TOOL, "codegate");
}

#[test]
fn a_run_that_prints_json_is_assessed_and_its_document_passed_through() {
    let (bin, path) = program(r#"[ "$1" = assess ] || exit 9; printf '{"rating":"B-"}'"#);
    let document = settled(&start(&bin, path, ASSESS_TIMEOUT));
    assert_eq!(document["status"], "assessed");
    assert_eq!(document["assessment"], json!({ "rating": "B-" }));
    assert_eq!(document["reason"], Value::Null);
}

#[test]
fn a_run_is_running_until_it_finishes() {
    let (bin, path) = program("sleep 2; printf '{}'");
    let run = start(&bin, path, ASSESS_TIMEOUT);
    let first = run.document();
    assert_eq!(first["status"], "running");
    assert_eq!(first["assessment"], Value::Null);
    assert_eq!(settled(&run)["status"], "assessed");
}

#[test]
fn a_failing_run_is_failed_with_its_stderr_and_no_assessment() {
    let (bin, path) = program("printf 'build failed\\n' >&2; exit 2");
    let document = settled(&start(&bin, path, ASSESS_TIMEOUT));
    assert_eq!(document["status"], "failed");
    assert_eq!(document["reason"], "codegate assess failed");
    assert!(
        document["stderr"]
            .as_str()
            .unwrap()
            .contains("build failed"),
        "{document}"
    );
    assert_eq!(document["assessment"], Value::Null);
}

#[test]
fn a_run_past_the_timeout_is_failed_and_says_so() {
    let (bin, path) = program("sleep 30");
    let started = Instant::now();
    let document = settled(&start(&bin, path, Duration::from_millis(300)));
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(document["status"], "failed");
    assert_eq!(
        document["reason"], "codegate assess timed out after 0.3 s",
        "{document}"
    );
    assert!(
        document["stderr"].as_str().unwrap().contains("timed out"),
        "{document}"
    );
    assert_eq!(document["assessment"], Value::Null);
}

#[test]
fn a_run_that_prints_no_json_is_failed() {
    let (bin, path) = program("printf 'not json'");
    let document = settled(&start(&bin, path, ASSESS_TIMEOUT));
    assert_eq!(document["status"], "failed");
    assert_eq!(
        document["reason"],
        "codegate assess printed no JSON document"
    );
    assert_eq!(document["assessment"], Value::Null);
}
