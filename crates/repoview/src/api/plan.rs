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

use axum::Router;
use axum::extract::{Extension, Path as UrlPath};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use repoview_sources::{Env, Output, TIMEOUT, run_output, truncate_diagnostic};
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
    let ran = tokio::task::spawn_blocking(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_output(&env, &program, &args, TIMEOUT)
    })
    .await;
    let Ok(Output {
        exit,
        stdout,
        stderr,
        status,
        ..
    }) = ran
    else {
        return tool_error(
            StatusCode::BAD_GATEWAY,
            None,
            &format!("{TOOL} did not finish"),
            None,
        );
    };
    // No status: a timeout or a spawn error, whose diagnostic is in `stderr`, and no stdout.
    if status.is_none() {
        return tool_error(StatusCode::BAD_GATEWAY, None, &stderr, None);
    }
    if exit == Some(0) && serde_json::from_slice::<serde::de::IgnoredAny>(&stdout).is_ok() {
        return ([(header::CONTENT_TYPE, "application/json")], stdout).into_response();
    }
    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    if exit == Some(0) {
        let mut text = format!("{TOOL} exited 0 without printing a JSON document");
        if !stderr.trim().is_empty() {
            text.push_str(": ");
            text.push_str(&stderr);
        }
        return tool_error(StatusCode::BAD_GATEWAY, Some(0), &text, Some(stdout));
    }
    let stderr = if stderr.trim().is_empty() {
        match exit {
            Some(code) => format!("{TOOL} exited with status {code}"),
            None => format!("{TOOL} was killed by a signal"),
        }
    } else {
        stderr
    };
    tool_error(StatusCode::BAD_GATEWAY, exit, &stderr, Some(stdout))
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

#[cfg(test)]
mod tests {
    use super::*;

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
