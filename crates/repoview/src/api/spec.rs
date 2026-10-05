//! `/api/spec/*`: every ESS specification root the `spec` source detects, answered by the `ess` on
//! `PATH`.
//!
//! | route | answer |
//! |---|---|
//! | `/api/spec/roots` | `[{ root, validate, ok }]`, `validate` being `ess specify validate` JSON |
//! | `/api/spec/roots/{root}/ir` | `ess specify compile --format json`, passed through |
//! | `/api/spec/roots/{root}/graph` | `ess specify graph --format json`, passed through |
//! | `/api/spec/roots/{root}/mermaid` | `{ "mermaid": <ess specify graph --format mermaid> }` |
//!
//! `{root}` may contain `/`, literal or `%2F`. The project root itself is the root `.`, spelt
//! [`ROOT_KEY`] (`~`) on the wire: a browser drops `.` and `%2e` segments before sending, so `.`
//! is never a key and a bare `<view>` names no root. A `{root}` that is not exactly one of the
//! detected roots is 404 before any `ess` process starts, whether or not `ess` is on `PATH`: the
//! roots come from the `spec` source's walk (`git ls-files`, or a bounded directory walk), never
//! from `ess`. For a detected root, a tool failure is 502 and a missing tool 503, both with the
//! body `{ "tool": "ess", "exit", "stderr" }` (plus `stdout` when the tool printed any).
//!
//! The project is the `Extension<Env>` layered over the router; every `ess` runs in its root
//! with its `PATH`.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use axum::Extension;
use axum::Router;
use axum::extract::Path as UrlPath;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use repoview_sources::{Env, TIMEOUT, detected_roots, truncate_diagnostic};
use serde_json::{Value, json};

use crate::server::AppState;

const TOOL: &str = "ess";

/// The routes `api::routes` merges. The project they read is the `Extension<Env>` that
/// `repoview open` layers over the router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/spec/roots", get(roots))
        .route("/api/spec/roots/{*rest}", get(root_view))
}

/// One of the three per-root documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Ir,
    Graph,
    Mermaid,
}

impl View {
    fn args(self, path_arg: &str) -> [&str; 5] {
        let (verb, format) = match self {
            View::Ir => ("compile", "json"),
            View::Graph => ("graph", "json"),
            View::Mermaid => ("graph", "mermaid"),
        };
        ["specify", verb, path_arg, "--format", format]
    }
}

/// The key that names the project root `.` in a URL. A detected root literally called `~` is
/// shadowed by it.
pub const ROOT_KEY: &str = "~";

/// `<key>/<view>` split at its last `/`, the key [`ROOT_KEY`] read as the root `.`. A bare
/// `<view>` and the key `.` name no root.
fn split_rest(rest: &str) -> Option<(&str, View)> {
    let (key, view) = rest.rsplit_once('/')?;
    let root = match key {
        ROOT_KEY => ".",
        "." => return None,
        key => key,
    };
    let view = match view {
        "ir" => View::Ir,
        "graph" => View::Graph,
        "mermaid" => View::Mermaid,
        _ => return None,
    };
    Some((root, view))
}

/// A JSON answer, or the original bytes of a tool's JSON output.
enum Answer {
    Json(StatusCode, Value),
    RawJson(String),
}

impl IntoResponse for Answer {
    fn into_response(self) -> Response {
        match self {
            Answer::Json(status, value) => (status, axum::Json(value)).into_response(),
            Answer::RawJson(text) => {
                ([(header::CONTENT_TYPE, "application/json")], text).into_response()
            }
        }
    }
}

async fn roots(Extension(env): Extension<Env>) -> Response {
    blocking(env, list_roots).await
}

async fn root_view(Extension(env): Extension<Env>, UrlPath(rest): UrlPath<String>) -> Response {
    blocking(env, move |env| view(env, &rest)).await
}

async fn blocking(env: Env, work: impl FnOnce(&Env) -> Answer + Send + 'static) -> Response {
    match tokio::task::spawn_blocking(move || work(&env)).await {
        Ok(answer) => answer.into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "spec request failed\n").into_response(),
    }
}

fn list_roots(env: &Env) -> Answer {
    let roots = detected_roots(env);
    if roots.is_empty() {
        return Answer::Json(StatusCode::OK, Value::Array(Vec::new()));
    }
    let Some(ess) = env.find_tool(TOOL) else {
        return missing();
    };
    let entries: Vec<Value> = thread::scope(|scope| {
        let handles: Vec<_> = roots
            .iter()
            .map(|root| scope.spawn(|| validate(env, &ess, root)))
            .collect();
        handles
            .into_iter()
            .zip(&roots)
            .map(|(handle, root)| {
                let (validate, ok) = handle
                    .join()
                    .unwrap_or_else(|_| (tool_error(None, "validate panicked", ""), false));
                json!({ "root": root, "validate": validate, "ok": ok })
            })
            .collect()
    });
    Answer::Json(StatusCode::OK, Value::Array(entries))
}

/// `ess specify validate` for one root: its JSON and whether it says the root is valid, or the
/// tool failure when it printed no JSON object.
fn validate(env: &Env, ess: &Path, root: &str) -> (Value, bool) {
    let path_arg = format!("--path={root}");
    let args = ["specify", "validate", &path_arg, "--format", "json"];
    match run(env, ess, &args, TIMEOUT) {
        Run::Exited {
            code,
            stdout,
            stderr,
        } => match serde_json::from_str::<Value>(&stdout) {
            Ok(value @ Value::Object(_)) => {
                let ok = code == Some(0) && value.get("valid") == Some(&Value::Bool(true));
                (value, ok)
            }
            _ => (tool_error(code, &stderr, &stdout), false),
        },
        Run::Failed(diagnostic) => (tool_error(None, &diagnostic, ""), false),
    }
}

fn view(env: &Env, rest: &str) -> Answer {
    let Some((root, view)) = split_rest(rest) else {
        return not_found();
    };
    // The 404 is decided by the roots walk alone: an undetected root starts no `ess` at all.
    if !detected_roots(env).iter().any(|detected| detected == root) {
        return not_found();
    }
    let Some(ess) = env.find_tool(TOOL) else {
        return missing();
    };
    let path_arg = format!("--path={root}");
    let (code, stdout, stderr) = match run(env, &ess, &view.args(&path_arg), TIMEOUT) {
        Run::Exited {
            code: Some(0),
            stdout,
            stderr,
        } => (0, stdout, stderr),
        Run::Exited {
            code,
            stdout,
            stderr,
        } => return bad_gateway(code, &stderr, &stdout),
        Run::Failed(diagnostic) => return bad_gateway(None, &diagnostic, ""),
    };
    match view {
        View::Mermaid => Answer::Json(StatusCode::OK, json!({ "mermaid": stdout })),
        View::Ir | View::Graph => match serde_json::from_str::<Value>(&stdout) {
            Ok(_) => Answer::RawJson(stdout),
            Err(_) => bad_gateway(Some(code), &stderr, &stdout),
        },
    }
}

fn tool_error(exit: Option<i32>, stderr: &str, stdout: &str) -> Value {
    let mut body = json!({
        "tool": TOOL,
        "exit": exit,
        "stderr": truncate_diagnostic(stderr),
    });
    if !stdout.is_empty() {
        body["stdout"] = json!(truncate_diagnostic(stdout));
    }
    body
}

fn bad_gateway(exit: Option<i32>, stderr: &str, stdout: &str) -> Answer {
    Answer::Json(StatusCode::BAD_GATEWAY, tool_error(exit, stderr, stdout))
}

fn missing() -> Answer {
    Answer::Json(
        StatusCode::SERVICE_UNAVAILABLE,
        tool_error(None, &format!("{TOOL} not found on PATH"), ""),
    )
}

fn not_found() -> Answer {
    Answer::Json(
        StatusCode::NOT_FOUND,
        json!({ "error": "not a detected specification root" }),
    )
}

/// One finished tool run.
enum Run {
    /// The tool exited; `code` is `None` when a signal ended it.
    Exited {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    /// It could not be started, or it timed out.
    Failed(String),
}

/// `program args…` in `env.root` with `env`'s `PATH`, no shell, bounded by `timeout` output
/// included, keeping stdout and the exit code on failure (`repoview_sources::run` keeps neither,
/// and `ess specify validate` prints its refusal as JSON on stdout with exit 1). The child leads
/// its own process group, which is killed at the deadline.
fn run(env: &Env, program: &Path, args: &[&str], timeout: Duration) -> Run {
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(env.root())
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = &env.path {
        command.env("PATH", path);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Run::Failed(format!("{}: {error}", program.display())),
    };
    let group = child.id();
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let timed_out = || {
        kill_group(group);
        Run::Failed(format!(
            "{} timed out after {} ms",
            program.display(),
            timeout.as_millis()
        ))
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let outcome = timed_out();
                let _ = child.wait();
                return outcome;
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                kill_group(group);
                let _ = child.wait();
                return Run::Failed(format!("{}: {error}", program.display()));
            }
        }
    };
    let remaining = || deadline.saturating_duration_since(Instant::now());
    let (Ok(stdout), Ok(stderr)) = (
        stdout.recv_timeout(remaining()),
        stderr.recv_timeout(remaining()),
    ) else {
        return timed_out();
    };
    Run::Exited {
        code: status.code(),
        stdout,
        stderr,
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

/// Read `pipe` to its end on a thread; the text arrives on the returned channel.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    receiver
}

#[cfg(test)]
mod tests {
    use super::{View, split_rest};

    #[test]
    fn split_rest_takes_the_view_from_the_last_segment() {
        assert_eq!(split_rest("ess/ir"), Some(("ess", View::Ir)));
        assert_eq!(split_rest("a/b/ess/graph"), Some(("a/b/ess", View::Graph)));
        assert_eq!(split_rest("~/mermaid"), Some((".", View::Mermaid)));
        assert_eq!(split_rest("mermaid"), None);
        assert_eq!(split_rest("./ir"), None);
        assert_eq!(split_rest("ess/ir/x"), None);
        assert_eq!(split_rest("ess"), None);
    }
}
