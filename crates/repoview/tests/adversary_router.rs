//! Adversary cases for story:server-skeleton: API path boundaries and static path traversal.

use std::borrow::Cow;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::{AssetSource, MemoryAssets};
use repoview::server::{AppState, TOKEN_HEADER, router};
use rust_embed::RustEmbed;
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

async fn get(state: AppState, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, Vec<u8>) {
    let mut request = Request::get(uri).header(header::HOST, HOST);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router(state)
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, body.to_vec())
}

/// Story wire contract: `/api/*` never falls back to the SPA. `/api/` with a valid token is an
/// unknown API path and must be 404, like `/api` and `/api/nope`.
#[tokio::test]
async fn api_with_trailing_slash_is_404_not_the_spa() {
    let assets = MemoryAssets::new().with("index.html", "<!doctype html><title>spa</title>");
    let token = "a".repeat(64);
    let (status, body) = get(state(assets), "/api/", &[(TOKEN_HEADER, &token)]).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "body: {}",
        String::from_utf8_lossy(&body)
    );
}

/// The same `rust-embed` setup `EmbeddedAssets` uses (`allow_missing`, read from disk in a debug
/// build), over a folder that exists in this tree.
#[derive(RustEmbed)]
#[folder = "tests"]
#[allow_missing = true]
struct DebugDist;

struct DebugAssets;

impl AssetSource for DebugAssets {
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        DebugDist::get(path).map(|file| file.data)
    }
}

/// `static_file` refuses `..` segments split on `/`, but `rust-embed` turns `\` into `/` before
/// joining. A static request (no token needed) must not read a file outside the asset folder.
#[tokio::test]
async fn backslash_dot_dot_does_not_escape_the_asset_folder() {
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    std::fs::write(&secret, "TOP-SECRET-OUTSIDE-WEB-DIST").unwrap();
    let link = outside.path().join("link");
    std::os::unix::fs::symlink(&secret, &link).unwrap();

    let absolute = std::fs::canonicalize(outside.path()).unwrap().join("link");
    let tail = absolute
        .to_str()
        .unwrap()
        .trim_start_matches('/')
        .replace('/', "\\");
    let uri = format!("/{}{tail}", "..\\".repeat(40));

    let (status, body) = get(state(DebugAssets), &uri, &[]).await;
    let text = String::from_utf8_lossy(&body);
    assert!(
        !text.contains("TOP-SECRET-OUTSIDE-WEB-DIST"),
        "{status} served a file outside the asset folder for {uri}"
    );
}
