//! The API routes the Plan pages read: `aep plan artifact … --format json`, run in the project
//! root, its stdout passed through unchanged.
//!
//! | answer | when |
//! |---|---|
//! | 200, aep's stdout byte for byte | `aep` exited 0 and printed JSON |
//! | 400 `{ "error" }` | `{id}` is not `^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$`; no process starts |
//! | 502 `{ "tool", "exit", "stderr", "stdout" }` | non-zero exit (`exit` is the code), a signal, a timeout or a spawn error (`exit` null), or exit 0 without JSON |
//! | 503 `{ "tool", "exit": null, "stderr" }` | no `aep` on `PATH` |
//!
//! `stdout` is kept on a 502 because `aep plan artifact validate` exits 1 for an invalid store and
//! prints the problems on stdout; without it the Board page could not show them.
//!
//! The project comes from an [`Env`] request extension; `repoview open` layers it over the router
//! (`axum::Extension(Env)`); a test adds its own.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{Extension, Path as UrlPath};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use repoview_sources::{Env, TIMEOUT, truncate_diagnostic};
use serde_json::{Value, json};

use crate::server::AppState;

const TOOL: &str = "aep";

/// What `{id}` must match; [`is_artifact_id`] decides it.
const ID_PATTERN: &str = "^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/plan/board", get(board))
        .route("/api/plan/artifacts", get(artifacts))
        .route("/api/plan/graph", get(graph))
        .route("/api/plan/validate", get(validate))
        .route("/api/plan/artifacts/{id}", get(show))
        .route("/api/plan/artifacts/{id}/history", get(history))
        .route("/api/plan/artifacts/{id}/explain", get(explain))
}

async fn board(Extension(env): Extension<Env>) -> Response {
    store_wide(env, "board").await
}

async fn artifacts(Extension(env): Extension<Env>) -> Response {
    store_wide(env, "list").await
}

async fn graph(Extension(env): Extension<Env>) -> Response {
    store_wide(env, "graph").await
}

async fn validate(Extension(env): Extension<Env>) -> Response {
    store_wide(env, "validate").await
}

async fn show(Extension(env): Extension<Env>, UrlPath(id): UrlPath<String>) -> Response {
    one_artifact(env, "show", id).await
}

async fn history(Extension(env): Extension<Env>, UrlPath(id): UrlPath<String>) -> Response {
    one_artifact(env, "history", id).await
}

async fn explain(Extension(env): Extension<Env>, UrlPath(id): UrlPath<String>) -> Response {
    one_artifact(env, "explain", id).await
}

async fn store_wide(env: Env, verb: &'static str) -> Response {
    answer(env, vec![verb.to_owned()]).await
}

async fn one_artifact(env: Env, verb: &'static str, id: String) -> Response {
    if !is_artifact_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("artifact id must match {ID_PATTERN}") })),
        )
            .into_response();
    }
    // `--` ends the options, so an id that starts with `-` is never read as one.
    answer(env, vec![verb.to_owned(), "--".to_owned(), id]).await
}

/// `^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$`: aep's identifier grammar
/// without `/`. Neither part may start with `.`, so `..` is never a whole part.
pub fn is_artifact_id(id: &str) -> bool {
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    let part = |text: &str| match text.as_bytes() {
        [first, rest @ ..] => allowed(*first) && rest.iter().all(|&b| allowed(b) || b == b'.'),
        [] => false,
    };
    id.split_once(':')
        .is_some_and(|(namespace, name)| part(namespace) && part(name))
}

/// Run `aep plan artifact <verb> --format json [rest…]` and turn the result into a response.
async fn answer(env: Env, verb_and_rest: Vec<String>) -> Response {
    let Some(program) = env.find_tool(TOOL) else {
        return tool_error(
            StatusCode::SERVICE_UNAVAILABLE,
            None,
            &format!("{TOOL} not found on PATH"),
            None,
        );
    };
    let mut args = vec!["plan".to_owned(), "artifact".to_owned()];
    let mut rest = verb_and_rest.into_iter();
    args.extend(rest.next());
    args.extend(["--format".to_owned(), "json".to_owned()]);
    args.extend(rest);
    let ran = tokio::task::spawn_blocking(move || run_tool(&env, &program, &args, TIMEOUT)).await;
    let Ok(ran) = ran else {
        return tool_error(
            StatusCode::BAD_GATEWAY,
            None,
            &format!("{TOOL} did not finish"),
            None,
        );
    };
    match ran {
        Ran::Exited {
            code: Some(0),
            stdout,
            stderr,
        } => {
            if serde_json::from_slice::<serde::de::IgnoredAny>(&stdout).is_ok() {
                ([(header::CONTENT_TYPE, "application/json")], stdout).into_response()
            } else {
                let mut text = format!("{TOOL} exited 0 without printing a JSON document");
                if !stderr.trim().is_empty() {
                    text.push_str(": ");
                    text.push_str(&stderr);
                }
                tool_error(
                    StatusCode::BAD_GATEWAY,
                    Some(0),
                    &text,
                    Some(String::from_utf8_lossy(&stdout).into_owned()),
                )
            }
        }
        Ran::Exited {
            code,
            stdout,
            stderr,
        } => {
            let stderr = if stderr.trim().is_empty() {
                match code {
                    Some(code) => format!("{TOOL} exited with status {code}"),
                    None => format!("{TOOL} was killed by a signal"),
                }
            } else {
                stderr
            };
            tool_error(
                StatusCode::BAD_GATEWAY,
                code,
                &stderr,
                Some(String::from_utf8_lossy(&stdout).into_owned()),
            )
        }
        Ran::Failed(diagnostic) => tool_error(StatusCode::BAD_GATEWAY, None, &diagnostic, None),
    }
}

/// `{ "tool", "exit", "stderr", "stdout"? }` with `status`; `stderr` cut to the diagnostic limit.
fn tool_error(
    status: StatusCode,
    exit: Option<i32>,
    stderr: &str,
    stdout: Option<String>,
) -> Response {
    let mut body = json!({
        "tool": TOOL,
        "exit": exit,
        "stderr": truncate_diagnostic(stderr),
    });
    if let (Some(stdout), Value::Object(map)) = (stdout, &mut body) {
        map.insert("stdout".to_owned(), Value::String(stdout));
    }
    (status, Json(body)).into_response()
}

/// What one run of a tool produced.
#[derive(Debug)]
enum Ran {
    /// The process ended; `code` is `None` when a signal ended it.
    Exited {
        code: Option<i32>,
        stdout: Vec<u8>,
        stderr: String,
    },
    /// It could not be started, or did not finish within the timeout.
    Failed(String),
}

/// Run `program args…` in `env.root` with `env`'s `PATH`, no shell, stdin closed.
///
/// The exit code matters here, which `repoview_sources::run` does not report, hence this runner.
/// Like that one, the child leads its own process group and the group is killed at the deadline,
/// so a grandchild holding a pipe open cannot keep the request waiting.
fn run_tool(env: &Env, program: &Path, args: &[String], timeout: Duration) -> Ran {
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(env.root())
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = env.search_path() {
        command.env("PATH", path);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Ran::Failed(format!("{}: {error}", program.display())),
    };
    let group = child.id();
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let timed_out = || {
        kill_group(group);
        Ran::Failed(format!(
            "{} timed out after {} ms",
            program.display(),
            timeout.as_millis()
        ))
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let ran = timed_out();
                let _ = child.wait();
                return ran;
            }
            Err(error) => {
                kill_group(group);
                let _ = child.wait();
                return Ran::Failed(format!("{}: {error}", program.display()));
            }
        }
    };
    let (Ok(stdout), Ok(stderr)) = (
        stdout.recv_timeout(deadline.saturating_duration_since(Instant::now())),
        stderr.recv_timeout(deadline.saturating_duration_since(Instant::now())),
    ) else {
        return timed_out();
    };
    Ran::Exited {
        code: status.code(),
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    }
}

/// SIGKILL to every process in the group `group` leads.
fn kill_group(group: u32) {
    if let Ok(group) = libc::pid_t::try_from(group) {
        // SAFETY: kill(2) with a negative pid signals the process group; it touches no memory.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
}

/// Read `pipe` to its end on a thread; the bytes arrive on the returned channel.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(bytes);
    });
    receiver
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn script(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("tool");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn run_script(body: &str, timeout: Duration) -> (Ran, Duration) {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), body);
        let env = Env::new(dir.path());
        // ETXTBSY: a concurrently forked test thread may still hold the write descriptor.
        for _ in 0..200 {
            let started = Instant::now();
            match run_tool(&env, &program, &[], timeout) {
                Ran::Failed(text) if text.contains("Text file busy") => {
                    thread::sleep(Duration::from_millis(10))
                }
                ran => return (ran, started.elapsed()),
            }
        }
        panic!("the script stayed busy");
    }

    #[test]
    fn the_exit_code_and_both_streams_are_reported() {
        let (ran, _) = run_script("printf out\nprintf err >&2\nexit 7", Duration::from_secs(5));
        match ran {
            Ran::Exited {
                code,
                stdout,
                stderr,
            } => {
                assert_eq!(code, Some(7));
                assert_eq!(stdout, b"out");
                assert_eq!(stderr, "err");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_hung_tool_is_killed_at_the_deadline_even_with_a_grandchild_holding_the_pipe() {
        let (ran, elapsed) = run_script("sleep 30 &\nsleep 30", Duration::from_millis(300));
        match ran {
            Ran::Failed(text) => assert!(text.contains("timed out after 300 ms"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    }

    #[test]
    fn a_grandchild_holding_the_pipe_after_exit_is_bounded_too() {
        let (ran, elapsed) = run_script("sleep 30 &\nexit 0", Duration::from_millis(300));
        assert!(matches!(ran, Ran::Failed(_)), "{ran:?}");
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    }

    #[test]
    fn the_id_pattern() {
        for good in [
            "story:plan-pages",
            "a:b",
            "0-9:z-",
            "-:-",
            "vision:O2",
            "A_b:c.D",
            "a..b:c.",
        ] {
            assert!(is_artifact_id(good), "{good}");
        }
        for bad in [
            "", ":", "a:", ":b", "a:b:c", "a b:c", "a;b:c", "..", "a:..", "..:a", ".a:b", "a:.b",
            "a/b:c", "a:b/c", "a+:b", "\u{e9}:a",
        ] {
            assert!(!is_artifact_id(bad), "{bad}");
        }
    }
}
