//! The static files the server answers non-API paths from.

use std::borrow::Cow;
use std::collections::HashMap;

use rust_embed::RustEmbed;

/// A set of static files addressed by relative path (`index.html`, `assets/app.js`).
pub trait AssetSource: Send + Sync {
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>>;
}

/// Files held in memory; tests inject one.
#[derive(Debug, Default, Clone)]
pub struct MemoryAssets {
    files: HashMap<String, Vec<u8>>,
}

impl MemoryAssets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, path: &str, body: impl Into<Vec<u8>>) -> Self {
        self.files.insert(path.to_owned(), body.into());
        self
    }
}

impl AssetSource for MemoryAssets {
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        self.files.get(path).map(|body| Cow::Owned(body.clone()))
    }
}

/// `web/dist`, embedded in release builds (`build.rs` refuses a release build without it) and
/// read from disk, possibly absent, in debug builds.
#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct Dist;

/// Digest of `web/dist` at compile time (`build.rs`); reading it here makes the compiler
/// invocation depend on the web build, so neither cargo nor a compiler cache reuses a crate
/// compiled against an older or absent web app.
pub const WEB_DIST_DIGEST: &str = env!("REPOVIEW_WEB_DIST_DIGEST");

/// The built web app.
pub struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        Dist::get(path).map(|file| file.data)
    }
}

/// The `Content-Type` for a static file, by extension.
pub fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// What `/` serves when there is no built web app (a debug build without `web/dist`).
pub const PLACEHOLDER: &str = "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
<title>repoview</title></head>\n<body>\n<h1>repoview</h1>\n<p>The web app is not built. Run \
<code>task build</code> (or <code>task dev</code> for the Vite dev server). The API is up at \
<code>/api/snapshot</code>.</p>\n</body>\n</html>\n";
