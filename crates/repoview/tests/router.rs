//! The router against an injected asset source and snapshot, without a socket.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::{AssetSource, MemoryAssets};
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use serde_json::json;
use tower::ServiceExt;

const PORT: u16 = 7480;
const HOST: &str = "127.0.0.1:7480";

fn state(assets: impl AssetSource + 'static) -> AppState {
    AppState {
        token: "a".repeat(64),
        port: PORT,
        assets: Arc::new(assets),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    }
}

async fn get(
    state: AppState,
    uri: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, String, Vec<u8>) {
    let mut request = Request::get(uri).header(header::HOST, HOST);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router(state)
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
    (status, content_type, body.to_vec())
}

fn spa() -> MemoryAssets {
    MemoryAssets::new()
        .with("index.html", "<!doctype html><title>spa</title>")
        .with("assets/app-abc.js", "console.log('app')")
}

#[tokio::test]
async fn injected_assets_are_served_with_their_content_types() {
    let (status, content_type, body) = get(state(spa()), "/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("text/html"), "{content_type}");
    assert_eq!(body, b"<!doctype html><title>spa</title>");

    let (status, content_type, body) = get(state(spa()), "/assets/app-abc.js", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        content_type.starts_with("text/javascript"),
        "{content_type}"
    );
    assert_eq!(body, b"console.log('app')");
}

#[tokio::test]
async fn unknown_static_path_falls_back_to_index_html() {
    let (status, content_type, body) = get(state(spa()), "/plan/board", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("text/html"));
    assert_eq!(body, b"<!doctype html><title>spa</title>");
}

fn spa_with_head() -> MemoryAssets {
    MemoryAssets::new()
        .with(
            "index.html",
            "<!doctype html><html><head><script src=\"./assets/app-abc.js\"></script></head></html>",
        )
        .with("assets/app-abc.js", "console.log('app')")
}

#[tokio::test]
async fn index_html_gets_a_base_href_at_root_and_through_the_fallback() {
    let expected = "<!doctype html><html><head><base href=\"/\"><script src=\"./assets/app-abc.js\"></script></head></html>";
    for path in ["/", "/a/b"] {
        let (status, content_type, body) = get(state(spa_with_head()), path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(content_type.starts_with("text/html"), "{path}");
        assert_eq!(String::from_utf8(body).unwrap(), expected, "{path}");
    }
    let (status, content_type, body) = get(state(spa_with_head()), "/assets/app-abc.js", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("text/javascript"));
    assert_eq!(body, b"console.log('app')");
}

#[tokio::test]
async fn an_existing_base_element_is_left_alone() {
    let index = "<!doctype html><html><head><base href=\"/app/\"></head></html>";
    let assets = MemoryAssets::new().with("index.html", index);
    let (_, _, body) = get(state(assets), "/a/b", &[]).await;
    assert_eq!(String::from_utf8(body).unwrap(), index);
}

#[tokio::test]
async fn without_an_asset_set_root_explains_task_build() {
    let (status, content_type, body) = get(state(MemoryAssets::new()), "/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("text/html"));
    assert!(String::from_utf8(body).unwrap().contains("task build"));
}

#[tokio::test]
async fn api_answers_the_injected_snapshot_with_the_token() {
    let token = "a".repeat(64);
    let (status, content_type, body) =
        get(state(spa()), "/api/snapshot", &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("application/json"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        json!({ "sources": [] })
    );
}

#[tokio::test]
async fn api_without_the_token_is_forbidden_and_never_falls_back() {
    let (status, _, _) = get(state(spa()), "/api/snapshot", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = get(state(spa()), "/api/nope", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let token = "a".repeat(64);
    let (status, _, _) = get(state(spa()), "/api/nope", &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = get(state(spa()), "/api", &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_host_header_is_forbidden() {
    let response = router(state(spa()))
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn tokens_are_64_hex_characters_and_differ_per_run() {
    let first = new_token();
    let second = new_token();
    assert_eq!(first.len(), 64);
    assert!(
        first
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_ne!(first, second);
}

// Correction round 1.

async fn status(uri: &str) -> StatusCode {
    get(state(spa()), uri, &[]).await.0
}

/// Finding 2: a static path with `\`, a NUL, an empty, `.` or `..` segment — literal or
/// percent-encoded — is 404 and never reaches the asset source.
#[tokio::test]
async fn static_paths_that_could_traverse_are_404() {
    for uri in [
        "/a/../index.html",
        "/./index.html",
        "/assets/./app-abc.js",
        "/..%2findex.html",
        "/%2e%2e/index.html",
        "/%2E%2e/index.html",
        "/assets%5c..%5capp-abc.js",
        "/assets\\app-abc.js",
        "/a%00b",
        "/assets//app-abc.js",
        "/%zz",
    ] {
        assert_eq!(status(uri).await, StatusCode::NOT_FOUND, "{uri}");
    }
    assert_eq!(status("/assets/app-abc.js").await, StatusCode::OK);
    assert_eq!(status("/plan/board").await, StatusCode::OK);
}

/// Finding 3: every `/api` path no route answers is 404 after the token check, never the SPA,
/// whatever the method.
#[tokio::test]
async fn unmatched_api_paths_are_404_never_the_spa() {
    let token = "a".repeat(64);
    for uri in ["/api", "/api/", "/api//", "/api/snapshot/", "/api/a/b"] {
        let (status, _, body) = get(state(spa()), uri, &[(TOKEN_HEADER, &token)]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(!String::from_utf8(body).unwrap().contains("spa"), "{uri}");
        let (status, _, _) = get(state(spa()), uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri} without the token");
    }
    let response = router(state(spa()))
        .oneshot(
            Request::post("/api/nope")
                .header(header::HOST, HOST)
                .header(TOKEN_HEADER, &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
