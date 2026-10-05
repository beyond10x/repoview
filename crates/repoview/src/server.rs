//! The local HTTP server: `127.0.0.1` only, a run token on `/api/*`, a `Host` check on everything.

use std::net::Ipv4Addr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;

use crate::assets::{AssetSource, PLACEHOLDER, content_type};

/// The header the SPA sends the run token in.
pub const TOKEN_HEADER: &str = "x-repoview-token";

/// Produces the snapshot document for `GET /api/snapshot`.
pub type SnapshotFn = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;

/// Everything a request handler needs.
#[derive(Clone)]
pub struct AppState {
    pub token: String,
    pub port: u16,
    pub assets: Arc<dyn AssetSource>,
    pub snapshot: SnapshotFn,
}

/// A fresh run token: 32 bytes from the OS RNG, hex-encoded.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random number generator is available");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Bind `127.0.0.1:port`. There is no other address.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
}

/// The application router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/snapshot", get(snapshot))
        .fallback(fallback)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

/// The `Host` check on every request, then the token check on `/api`.
async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if !host_allowed(request.headers(), request.uri(), state.port) {
        return forbidden();
    }
    let path = request.uri().path();
    if (path == "/api" || path.starts_with("/api/")) && !token_valid(&request, &state.token) {
        return forbidden();
    }
    next.run(request).await
}

fn host_allowed(headers: &HeaderMap, uri: &Uri, port: u16) -> bool {
    let host = match headers.get(header::HOST) {
        Some(value) => value.to_str().ok(),
        None => uri.authority().map(|authority| authority.as_str()),
    };
    let Some(host) = host else {
        return false;
    };
    ["127.0.0.1", "localhost"]
        .iter()
        .any(|name| host.eq_ignore_ascii_case(&format!("{name}:{port}")))
}

fn token_valid(request: &Request, expected: &str) -> bool {
    let presented = request
        .headers()
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            request.uri().query().and_then(|query| {
                query
                    .split('&')
                    .filter_map(|pair| pair.split_once('='))
                    .find(|(name, _)| *name == "token")
                    .map(|(_, value)| value)
            })
        });
    presented.is_some_and(|token| bool::from(token.as_bytes().ct_eq(expected.as_bytes())))
}

fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, "forbidden\n").into_response()
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found\n").into_response()
}

async fn snapshot(State(state): State<AppState>) -> Response {
    let produce = state.snapshot.clone();
    match tokio::task::spawn_blocking(move || produce()).await {
        Ok(value) => Json(value).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "snapshot failed\n").into_response(),
    }
}

/// Every request no route answers. An `/api` path is 404 (the guard has already checked the
/// token), never the SPA; anything else is a static file.
async fn fallback(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    let path = uri.path();
    if path == "/api" || path.starts_with("/api/") {
        return not_found();
    }
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    match asset_path(path) {
        Some(path) => static_file(&state, &path),
        None => not_found(),
    }
}

/// The asset path a request path names, percent-decoded, or `None` when it could leave the
/// asset folder: a `\`, a NUL, or an empty, `.` or `..` segment, literal or encoded.
fn asset_path(path: &str) -> Option<String> {
    let path = path.strip_prefix('/').unwrap_or(path);
    let decoded = percent_decode(path)?;
    if decoded.contains(['\\', '\0']) {
        return None;
    }
    let segments = decoded.strip_suffix('/').unwrap_or(&decoded);
    if !segments.is_empty()
        && segments
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    Some(decoded)
}

/// `%XX` escapes decoded; `None` for a malformed escape or a result that is not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// A file of the web app, else its `index.html` (client-side routes), else the placeholder.
fn static_file(state: &AppState, path: &str) -> Response {
    if !path.is_empty()
        && let Some(body) = state.assets.get(path)
    {
        return (
            [(header::CONTENT_TYPE, content_type(path))],
            body.into_owned(),
        )
            .into_response();
    }
    let html = [(header::CONTENT_TYPE, content_type("index.html"))];
    match state.assets.get("index.html") {
        Some(body) => (html, with_base_href(&body)).into_response(),
        None => (html, PLACEHOLDER).into_response(),
    }
}

/// `index.html` with `<base href="/">` right after its `<head>` tag, unless it already has a
/// `<base`. The SPA is built with relative asset URLs (`./assets/…`), which a client-side route
/// such as `/a/b` would otherwise resolve under `/a/`.
fn with_base_href(index: &[u8]) -> Vec<u8> {
    let lower = index.to_ascii_lowercase();
    let has_base = lower.windows(5).any(|window| window == b"<base");
    let head_end = lower
        .windows(5)
        .enumerate()
        .filter(|(_, window)| *window == b"<head")
        .map(|(at, _)| at + 5)
        .find(|&after| matches!(lower.get(after), Some(b'>' | b' ' | b'\t' | b'\n' | b'\r')))
        .and_then(|after| {
            lower[after..]
                .iter()
                .position(|&b| b == b'>')
                .map(|gt| after + gt + 1)
        });
    match head_end {
        Some(insert_at) if !has_base => {
            let mut out = Vec::with_capacity(index.len() + 16);
            out.extend_from_slice(&index[..insert_at]);
            out.extend_from_slice(b"<base href=\"/\">");
            out.extend_from_slice(&index[insert_at..]);
            out
        }
        _ => index.to_vec(),
    }
}
