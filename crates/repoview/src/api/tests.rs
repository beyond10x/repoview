//! story:page-frame acceptance 1: a route an API module adds is merged into `server::router`, so it
//! gets the `Host` check, the token check and the `/api` 404 fallback.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use tower::ServiceExt;

use super::probe;
use crate::assets::MemoryAssets;
use crate::server::{AppState, TOKEN_HEADER, router};

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

async fn get(uri: &str, host: &str, headers: &[(&str, &str)]) -> (StatusCode, String) {
    let mut request = Request::get(uri).header(header::HOST, host);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router(state())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn a_module_route_is_403_without_the_token_and_200_with_it() {
    let token = token();

    let (status, body) = get(probe::PATH, HOST, &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "no token: {body}");

    let wrong = "b".repeat(64);
    let (status, _) = get(probe::PATH, HOST, &[(TOKEN_HEADER, &wrong)]).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "wrong token");

    let (status, body) = get(probe::PATH, HOST, &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::OK, "header token: {body}");
    assert_eq!(body, probe::BODY);

    let (status, body) = get(&format!("{}?token={token}", probe::PATH), HOST, &[]).await;
    assert_eq!(status, StatusCode::OK, "query token: {body}");
    assert_eq!(body, probe::BODY);
}

#[tokio::test]
async fn a_module_route_refuses_a_foreign_host_even_with_the_token() {
    let token = token();
    let (status, _) = get(probe::PATH, "evil.example:7480", &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_path_below_a_module_route_is_the_api_404_never_the_spa() {
    let token = token();
    let below = format!("{}/nope", probe::PATH);
    let (status, body) = get(&below, HOST, &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!body.contains("spa"), "{body}");
    let (status, _) = get(&below, HOST, &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
