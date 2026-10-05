//! `repoview export`: a static copy of every page.
//!
//! The copy is the embedded `index.html` with `<meta name="repoview-mode" content="static">`, the
//! embedded assets, and one JSON file per API answer the pages read, at `data/<api path>.json`
//! (the static mapping in `web/src/api/client.ts`). Every answer comes from the real router
//! ([`crate::server::router`]), asked in process with the run token; a non-2xx answer is written
//! as `data/<api path>.error.json` holding `{ status, body }`. `data/export.json` lists every
//! route with its status and every file the export wrote, which is what `--force` removes.
//!
//! Before anything is written each answer is rewritten so that the copy carries no run token, no
//! absolute path under the project root or the exporting user's home directory (they become `.`
//! and `~`), and no `tool_path` beyond the tool's file name.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Extension;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use repoview_sources::Env;
use rust_embed::RustEmbed;
use serde_json::{Map, Value, json};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tower::ServiceExt;

use crate::assets::{AssetSource, EmbeddedAssets, MemoryAssets};
use crate::project::snapshot;
use crate::server::{AppState, TOKEN_HEADER, router};

/// The manifest every export writes, relative to `--out`.
pub const MANIFEST: &str = "data/export.json";

/// The port the in-process requests name in `Host`; nothing listens on it.
const PORT: u16 = 7480;

/// How many answers are produced at once.
const CONCURRENCY: usize = 8;

/// The routes every export asks, in this order, before the per-item ones.
const FIXED: [&str; 9] = [
    "snapshot",
    "plan/board",
    "plan/artifacts",
    "plan/graph",
    "plan/validate",
    "spec/roots",
    "quality",
    "vcs",
    "docs",
];

/// What `repoview export` was asked to do.
#[derive(Debug, Clone)]
pub struct Options {
    /// The directory to write; absent, empty, or (with `force`) a previous export.
    pub out: PathBuf,
    /// Replace a previous export: remove the files its manifest lists, then write.
    pub force: bool,
    /// The run token the in-process requests carry; never written.
    pub token: String,
    /// The exporting user's home directory; no path under it is written.
    pub home: Option<PathBuf>,
}

/// The web app to copy: its `index.html` (absent when the app is not built) and every file.
#[derive(Debug, Clone)]
pub struct Site {
    index: Option<Vec<u8>>,
    files: Vec<(String, Vec<u8>)>,
}

/// The names of the embedded `web/dist` files; their bytes come from [`EmbeddedAssets`], so the
/// binary carries them once.
#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
#[metadata_only = true]
struct DistNames;

impl Site {
    /// A site of `index` and `files` (relative paths; an `index.html` among them is ignored).
    pub fn new(index: Vec<u8>, files: Vec<(String, Vec<u8>)>) -> Site {
        Site {
            index: Some(index),
            files,
        }
    }

    /// The web app embedded in this binary (`web/dist`).
    pub fn embedded() -> Site {
        let index = EmbeddedAssets
            .get("index.html")
            .map(|body| body.into_owned());
        let files = DistNames::iter()
            .filter_map(|name| {
                let body = EmbeddedAssets.get(&name)?.into_owned();
                Some((name.into_owned(), body))
            })
            .collect();
        Site { index, files }
    }
}

/// Why an export did not happen.
#[derive(Debug)]
pub enum Error {
    /// `--out` is not a directory the export may write into; nothing was touched.
    Refused(String),
    /// Anything else.
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(message) | Error::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

/// What an export wrote.
#[derive(Debug, Clone)]
pub struct Summary {
    /// Every route asked, with the status it answered.
    pub routes: Vec<(String, u16)>,
    /// Items whose route the pages could not request (an empty, `.` or `..` segment), so none was
    /// asked.
    pub skipped: Vec<String>,
}

/// One route's answer, already rewritten by [`Scrub`].
struct Answer {
    path: String,
    status: u16,
    body: Value,
}

/// Export the project `env` names into `options.out`.
pub fn export(env: &Env, site: &Site, options: &Options) -> Result<Summary, Error> {
    let existing = resolve_out(&options.out)?;
    let previous = match &existing {
        Some(out) => check_out(out, options.force)?,
        None => None,
    };
    let index = site.index.as_deref().ok_or_else(|| {
        Error::Failed(
            "the web app is not built (no web/dist/index.html in this binary); run `task build`"
                .to_owned(),
        )
    })?;
    let index = static_index(index)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| Error::Failed(format!("runtime: {error}")))?;
    let (raw, skipped) = runtime.block_on(collect(env.clone(), options.token.clone()));
    drop(runtime);

    let mut tool_paths = Vec::new();
    for answer in &raw {
        tool_paths_in(&answer.body, &mut tool_paths);
        // `/api/quality` names every `codegate` it passed over in `skipped`.
        if answer.path == "quality"
            && let Some(skipped) = answer.body.get("skipped").and_then(Value::as_array)
        {
            tool_paths.extend(skipped.iter().filter_map(Value::as_str).map(str::to_owned));
        }
    }
    let scrub = Scrub::new(env.root(), options.home.as_deref(), &options.token)
        .tools(tool_paths.iter().map(String::as_str));
    let answers: Vec<Answer> = raw
        .into_iter()
        .map(|answer| Answer {
            body: scrub.value(answer.body),
            ..answer
        })
        .collect();

    let mut writes: Vec<(String, Vec<u8>)> = vec![("index.html".to_owned(), index)];
    for (name, body) in &site.files {
        if name != "index.html" {
            writes.push((name.clone(), body.clone()));
        }
    }
    for answer in &answers {
        let file = if (200..300).contains(&answer.status) {
            format!("data/{}.json", answer.path)
        } else {
            format!("data/{}.error.json", answer.path)
        };
        let body = if (200..300).contains(&answer.status) {
            answer.body.clone()
        } else {
            json!({ "status": answer.status, "body": answer.body })
        };
        writes.push((file, pretty(&body)?));
    }
    let mut seen = BTreeSet::new();
    for (name, _) in &writes {
        if relative(name).is_none() || name == MANIFEST {
            return Err(Error::Failed(format!("refusing to write {name:?}")));
        }
        if !seen.insert(name.as_str()) {
            return Err(Error::Failed(format!("two answers map to the file {name}")));
        }
    }

    let routes: Vec<(String, u16)> = answers
        .iter()
        .map(|answer| (answer.path.clone(), answer.status))
        .collect();
    let mut listed: Vec<&str> = writes.iter().map(|(name, _)| name.as_str()).collect();
    listed.push(MANIFEST);
    let manifest = json!({
        "repoview_version": env!("CARGO_PKG_VERSION"),
        "exported_at": rfc3339(SystemTime::now()),
        "routes": routes
            .iter()
            .map(|(path, status)| json!({ "path": path, "status": status }))
            .collect::<Vec<_>>(),
        "files": listed,
    });

    // Nothing is removed or written while any target sits below a symlink inside --out.
    if let Some(out) = &existing {
        let names = writes.iter().map(|(name, _)| name.as_str());
        for name in names.chain([MANIFEST]) {
            refuse_symlink_between(out, name)?;
        }
    }
    if let Some(previous) = previous
        && let Some(out) = &existing
    {
        remove_previous(out, &previous)?;
    }
    let out = match existing {
        Some(out) => out,
        None => {
            fs::create_dir_all(&options.out).map_err(|error| {
                Error::Failed(format!("create {}: {error}", options.out.display()))
            })?;
            resolve_out(&options.out)?
                .ok_or_else(|| Error::Failed(format!("{} vanished", options.out.display())))?
        }
    };
    // The manifest goes first, so even an export cut short can be replaced with `--force`.
    write(&out, MANIFEST, &pretty(&manifest)?)?;
    for (name, body) in &writes {
        write(&out, name, body)?;
    }
    Ok(Summary { routes, skipped })
}

/// Every `tool_path` string in `value`, appended to `out`.
fn tool_paths_in(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "tool_path"
                    && let Value::String(path) = value
                {
                    out.push(path.clone());
                }
                tool_paths_in(value, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| tool_paths_in(item, out)),
        _ => {}
    }
}

/// `--out` canonicalized, or `None` when it does not exist. A symlink or a non-directory is
/// refused.
fn resolve_out(out: &Path) -> Result<Option<PathBuf>, Error> {
    // `site/` names the symlink's target; `site` names the symlink.
    let out = &without_trailing_slashes(out);
    let meta = match fs::symlink_metadata(out) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Refused(format!("--out {}: {error}", out.display()))),
    };
    if meta.file_type().is_symlink() {
        return Err(Error::Refused(format!(
            "--out {} is a symlink; name the directory itself",
            out.display()
        )));
    }
    if !meta.is_dir() {
        return Err(Error::Refused(format!(
            "--out {} is not a directory",
            out.display()
        )));
    }
    fs::canonicalize(out)
        .map(Some)
        .map_err(|error| Error::Refused(format!("--out {}: {error}", out.display())))
}

/// `path` without trailing `/`, unless it is only slashes.
fn without_trailing_slashes(path: &Path) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    let end = bytes
        .iter()
        .rposition(|&b| b != b'/')
        .map_or(1.min(bytes.len()), |at| at + 1);
    PathBuf::from(std::ffi::OsStr::from_bytes(&bytes[..end]))
}

/// [`Error::Refused`] when a directory between `out` and `out/name` is a symlink. Each step is
/// a single component, never spelt with a trailing `/`, so a symlink is seen, not followed.
fn refuse_symlink_between(out: &Path, name: &str) -> Result<(), Error> {
    let mut at = without_trailing_slashes(out);
    let mut components = Path::new(name).components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            break;
        }
        at.push(component);
        match fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(Error::Refused(format!(
                    "{} is a symlink inside --out {}; the export neither removes nor writes \
                     through it",
                    at.display(),
                    out.display()
                )));
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    Ok(())
}

/// `Ok(None)` when `out` is absent or empty, `Ok(Some(files))` with the previous export's file
/// list when `force` is set and `out` holds one, else [`Error::Refused`].
fn check_out(out: &Path, force: bool) -> Result<Option<Vec<String>>, Error> {
    let entries = match fs::read_dir(out) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::Refused(format!("--out {}: {error}", out.display())));
        }
    };
    if entries.count() == 0 {
        return Ok(None);
    }
    if !force {
        return Err(Error::Refused(format!(
            "--out {} is not empty; pass --force to replace a previous export there",
            out.display()
        )));
    }
    refuse_symlink_between(out, MANIFEST)?;
    let manifest = out.join(MANIFEST);
    let not_an_export = |why: String| {
        Error::Refused(format!(
            "--out {} is not empty and {MANIFEST} {why}, so no previous export is known there; \
             --force removes only what a previous export wrote",
            out.display()
        ))
    };
    let text = fs::read_to_string(&manifest).map_err(|error| not_an_export(error.to_string()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| not_an_export(format!("is not JSON ({error})")))?;
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .filter(|_| value.get("repoview_version").is_some_and(Value::is_string))
        .ok_or_else(|| not_an_export("is not a repoview export manifest".to_owned()))?;
    let mut list = Vec::with_capacity(files.len());
    for file in files {
        let name = file
            .as_str()
            .filter(|name| relative(name).is_some())
            .ok_or_else(|| not_an_export(format!("lists the file {file}")))?;
        refuse_symlink_between(out, name)?;
        list.push(name.to_owned());
    }
    Ok(Some(list))
}

/// Remove every file of the previous export, then every directory that removal left empty.
fn remove_previous(out: &Path, files: &[String]) -> Result<(), Error> {
    let mut directories = BTreeSet::new();
    for name in files {
        let path = out.join(name);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_file() => fs::remove_file(&path)
                .map_err(|error| Error::Failed(format!("remove {}: {error}", path.display())))?,
            _ => continue,
        }
        let mut parent = Path::new(name).parent();
        while let Some(dir) = parent.filter(|dir| !dir.as_os_str().is_empty()) {
            directories.insert(dir.to_path_buf());
            parent = dir.parent();
        }
    }
    // Deepest first; a directory that still holds something stays.
    let mut directories: Vec<PathBuf> = directories.into_iter().collect();
    directories.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in directories {
        let _ = fs::remove_dir(out.join(dir));
    }
    Ok(())
}

/// `name` as a relative path of normal components, or `None`.
fn relative(name: &str) -> Option<&Path> {
    let path = Path::new(name);
    let normal = !name.is_empty()
        && !name.contains('\0')
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    normal.then_some(path)
}

/// Write `out/name` without following a symlink: the bytes go to a new file beside it, which is
/// then renamed over `name` (replacing a symlink there, never its target).
fn write(out: &Path, name: &str, body: &[u8]) -> Result<(), Error> {
    let path = out.join(name);
    let parent = path.parent().unwrap_or(out);
    fs::create_dir_all(parent)
        .map_err(|error| Error::Failed(format!("create {}: {error}", parent.display())))?;
    refuse_symlink_between(out, name)?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let failed =
        |what: &str, error: io::Error| Error::Failed(format!("{what} {}: {error}", path.display()));
    let mut attempt = 0u32;
    let (temporary, mut file) = loop {
        let candidate = parent.join(format!(
            ".{file_name}.repoview-{}-{attempt}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempt < 100 => {
                attempt += 1;
            }
            Err(error) => return Err(failed("create a temporary file for", error)),
        }
    };
    let written = file
        .write_all(body)
        .and_then(|()| fs::rename(&temporary, &path));
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(failed("write", error));
    }
    Ok(())
}

fn pretty(value: &Value) -> Result<Vec<u8>, Error> {
    let mut text =
        serde_json::to_vec_pretty(value).map_err(|error| Error::Failed(error.to_string()))?;
    text.push(b'\n');
    Ok(text)
}

/// `index` with its `repoview-mode` meta set to `static`.
fn static_index(index: &[u8]) -> Result<Vec<u8>, Error> {
    let text = std::str::from_utf8(index)
        .map_err(|_| Error::Failed("the embedded index.html is not UTF-8".to_owned()))?;
    let missing = || {
        Error::Failed(
            "the embedded index.html has no <meta name=\"repoview-mode\" content=\"server\">"
                .to_owned(),
        )
    };
    let name_at = text.find("name=\"repoview-mode\"").ok_or_else(missing)?;
    let start = text[..name_at].rfind("<meta").ok_or_else(missing)?;
    let end = name_at + text[name_at..].find('>').ok_or_else(missing)?;
    let tag = &text[start..end];
    let tag = if tag.contains("content=\"server\"") {
        tag.replacen("content=\"server\"", "content=\"static\"", 1)
    } else if tag.contains("content=\"static\"") {
        tag.to_owned()
    } else {
        return Err(missing());
    };
    Ok(format!("{}{tag}{}", &text[..start], &text[end..]).into_bytes())
}

/// Ask every route the pages read: [`FIXED`], then each artifact's, root's and present
/// document's routes, named by those answers.
async fn collect(env: Env, token: String) -> (Vec<Answer>, Vec<String>) {
    let snapshot_env = env.clone();
    let state = AppState {
        token: token.clone(),
        port: PORT,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(move || {
            serde_json::to_value(snapshot(&snapshot_env)).expect("a snapshot serialises to JSON")
        }),
    };
    let app = router(state).layer(Extension(env));
    let token: Arc<str> = Arc::from(token);

    let mut answers = ask(&app, &token, FIXED.map(str::to_owned).to_vec()).await;
    let mut derived = Vec::new();
    let mut skipped = Vec::new();
    let mut push = |path: String| {
        if requestable(&path) {
            derived.push(path);
        } else {
            skipped.push(path);
        }
    };
    for answer in &answers {
        if !(200..300).contains(&answer.status) {
            continue;
        }
        let Some(items) = answer.body.as_array() else {
            continue;
        };
        match answer.path.as_str() {
            "plan/artifacts" => {
                for id in items.iter().filter_map(|item| item.get("id")?.as_str()) {
                    push(format!("plan/artifacts/{id}"));
                    push(format!("plan/artifacts/{id}/history"));
                    push(format!("plan/artifacts/{id}/explain"));
                }
            }
            "spec/roots" => {
                for root in items.iter().filter_map(|item| item.get("root")?.as_str()) {
                    // `web/src/api/spec.ts` `rootKey`: the root `.` is spelt `~`.
                    let key = if root == "." { "~" } else { root };
                    for view in ["ir", "graph", "mermaid"] {
                        push(format!("spec/roots/{key}/{view}"));
                    }
                }
            }
            "docs" => {
                let present = items
                    .iter()
                    .filter(|item| item.get("present") == Some(&Value::Bool(true)))
                    .filter_map(|item| item.get("name")?.as_str());
                for name in present {
                    push(format!("docs/{name}"));
                }
            }
            _ => {}
        }
    }
    answers.extend(ask(&app, &token, derived).await);
    (answers, skipped)
}

/// Whether the SPA would request `path`: `web/src/api/client.ts` `encodePath` refuses a path
/// with an empty, `.` or `..` segment.
fn requestable(path: &str) -> bool {
    !path.contains('\0')
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

/// The answers to `paths`, in order, at most [`CONCURRENCY`] at a time.
async fn ask(app: &Router, token: &Arc<str>, paths: Vec<String>) -> Vec<Answer> {
    let permits = Arc::new(Semaphore::new(CONCURRENCY));
    let mut tasks = JoinSet::new();
    for (at, path) in paths.into_iter().enumerate() {
        let app = app.clone();
        let token = token.clone();
        let permits = permits.clone();
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await;
            (at, get(app, &token, path).await)
        });
    }
    let mut answers: Vec<(usize, Answer)> = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        if let Ok(answer) = joined {
            answers.push(answer);
        }
    }
    answers.sort_by_key(|(at, _)| *at);
    answers.into_iter().map(|(_, answer)| answer).collect()
}

/// `GET /api/<path>` on the router, `path` encoded segment by segment as the SPA encodes it.
async fn get(app: Router, token: &str, path: String) -> Answer {
    let uri = format!("/api/{}", encode_path(&path));
    let request = Request::get(&uri)
        .header(header::HOST, format!("127.0.0.1:{PORT}"))
        .header(TOKEN_HEADER, token)
        .body(Body::empty());
    let failed = |status: u16, message: String| Answer {
        path: path.clone(),
        status,
        body: Value::String(message),
    };
    let request = match request {
        Ok(request) => request,
        Err(error) => return failed(400, format!("request {uri}: {error}")),
    };
    let response = match app.oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    };
    let status = response.status().as_u16();
    let bytes = match to_bytes(response.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => return failed(500, format!("reading the answer to {uri}: {error}")),
    };
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    Answer { path, status, body }
}

/// `path` with each segment encoded as `encodeURIComponent` encodes it.
fn encode_path(path: &str) -> String {
    path.split('/')
        .map(encode_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_component(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// What an answer may not carry: the run token, the project root and the home directory as
/// absolute paths. The root becomes `.` and the home `~`, each only as a whole path component
/// (`/srv/a` is not rewritten inside `/srv/ab`). Every tool path named with [`Scrub::tools`]
/// becomes the tool's file name wherever it appears, and a `tool_path` value always does.
#[derive(Debug, Clone)]
pub struct Scrub {
    /// `(absolute path, replacement)`, the root's spellings first, longest first within each.
    paths: Vec<(String, &'static str)>,
    /// `(tool path, file name)`, longest first.
    tools: Vec<(String, String)>,
    token: String,
}

impl Scrub {
    pub fn new(root: &Path, home: Option<&Path>, token: &str) -> Scrub {
        let mut paths = Vec::new();
        for (path, replacement) in [(Some(root), "."), (home, "~")] {
            let Some(path) = path else { continue };
            let mut spellings: Vec<String> =
                [Some(path.to_path_buf()), fs::canonicalize(path).ok()]
                    .into_iter()
                    .flatten()
                    .filter_map(|path| {
                        path.to_str()
                            .map(|text| text.trim_end_matches('/').to_owned())
                    })
                    // `/` or an empty or relative path names no directory worth hiding.
                    .filter(|text| text.starts_with('/') && text.len() > 1)
                    .collect();
            spellings.sort_by_key(|text| std::cmp::Reverse(text.len()));
            spellings.dedup();
            paths.extend(spellings.into_iter().map(|text| (text, replacement)));
        }
        Scrub {
            paths,
            tools: Vec::new(),
            token: token.to_owned(),
        }
    }

    /// Also cut each of `paths` (a tool's path, as a `tool_path` names it) to its file name
    /// wherever it appears, such as in a diagnostic that starts `<path> exited with`.
    pub fn tools<'a>(mut self, paths: impl IntoIterator<Item = &'a str>) -> Scrub {
        for path in paths {
            let Some(name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path != name && !self.tools.iter().any(|(known, _)| known == path) {
                self.tools.push((path.to_owned(), name.to_owned()));
            }
        }
        self.tools
            .sort_by_key(|(path, _)| std::cmp::Reverse(path.len()));
        self
    }

    /// `text` with the token and every listed path rewritten.
    pub fn text(&self, text: &str) -> String {
        let mut text = if self.token.is_empty() {
            text.to_owned()
        } else {
            text.replace(&self.token, "[token]")
        };
        for (path, name) in &self.tools {
            text = replace_component(&text, path, name, Left::WholePath);
        }
        for (path, replacement) in &self.paths {
            text = replace_component(&text, path, replacement, Left::Boundary);
        }
        text
    }

    /// `value` with every string and key rewritten by [`Scrub::text`], and every `tool_path`
    /// string cut to its file name.
    pub fn value(&self, value: Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.text(&text)),
            Value::Array(items) => {
                Value::Array(items.into_iter().map(|item| self.value(item)).collect())
            }
            Value::Object(map) => {
                let mut out = Map::with_capacity(map.len());
                for (key, value) in map {
                    let value = match value {
                        Value::String(path) if key == "tool_path" => {
                            let name = Path::new(&path)
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            Value::String(self.text(&name))
                        }
                        value => self.value(value),
                    };
                    out.insert(self.text(&key), value);
                }
                Value::Object(out)
            }
            other => other,
        }
    }
}

/// `text` with every occurrence of `path` that ends a path component replaced.
fn replace_component(text: &str, path: &str, replacement: &str, left: Left) -> String {
    let mut out = String::with_capacity(text.len());
    // `text[..from]` is settled; the next match is looked for from `search`.
    let mut from = 0;
    let mut search = 0;
    while let Some(found) = text[search..].find(path) {
        let at = search + found;
        let end = at + path.len();
        let ends_whole = ends_a_path(&text[end..]);
        let starts_whole = text[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_name_char(c));
        let start = match (ends_whole, starts_whole, left) {
            (true, true, _) => Some(at),
            // The tool path is the tail of a longer path (`/usr/bin/git` for `/bin/git`): the
            // whole longer path goes, so neither it nor the tool path is left half-rewritten.
            (true, false, Left::WholePath) => {
                let token = text[from..at]
                    .char_indices()
                    .rev()
                    .take_while(|&(_, c)| is_name_char(c) || c == '/')
                    .last()
                    .map_or(at, |(index, _)| from + index);
                Some(token)
            }
            _ => None,
        };
        match start {
            Some(start) => {
                out.push_str(&text[from..start]);
                out.push_str(replacement);
                from = end;
                search = end;
            }
            None => search = at + text[at..].chars().next().map_or(1, char::len_utf8),
        }
    }
    out.push_str(&text[from..]);
    out
}

/// How [`replace_component`] treats a match whose left side continues a name.
#[derive(Debug, Clone, Copy)]
enum Left {
    /// It is no match: `/srv/u` in `/mnt/srv/u` is not the path `/srv/u`.
    Boundary,
    /// The whole path it ends is replaced.
    WholePath,
}

/// A character that continues a file name: a match ending or starting next to one is not a
/// whole path (`/srv/p` in `/srv/p.git`, `/srv/u` in `/srv/ux`).
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '-' | '_')
}

/// Whether a path ends where `after` begins: at the end of the text, before a character that is
/// not a name character, or before a run of `.` that is itself followed by one of those, as
/// punctuation after a path (`/srv/u.`, `/srv/u.)`, `/srv/u...`) is; `/srv/p.git` and
/// `/srv/p.next` go on.
fn ends_a_path(after: &str) -> bool {
    let rest = after.trim_start_matches('.');
    rest.chars().next().is_none_or(|c| !is_name_char(c))
}

/// `time` as `YYYY-MM-DDTHH:MM:SSZ`.
fn rfc3339(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let of_day = seconds % 86_400;
    // Howard Hinnant's days-from-civil, inverted.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_formats_utc() {
        assert_eq!(rfc3339(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let leap = UNIX_EPOCH + std::time::Duration::from_secs(951_782_400 + 3_723);
        assert_eq!(rfc3339(leap), "2000-02-29T01:02:03Z");
    }

    #[test]
    fn encode_component_matches_encode_uri_component() {
        assert_eq!(encode_component("story:x"), "story%3Ax");
        assert_eq!(encode_component("my root"), "my%20root");
        assert_eq!(encode_component("~-_.!*'()"), "~-_.!*'()");
        assert_eq!(encode_component("ä/%"), "%C3%A4%2F%25");
        assert_eq!(encode_path("spec/roots/~/ir"), "spec/roots/~/ir");
    }

    #[test]
    fn static_index_switches_the_mode_meta_only() {
        let index = b"<head><meta name=\"repoview-mode\" content=\"server\" /><meta content=\"server\" name=\"x\"></head>";
        let out = String::from_utf8(static_index(index).unwrap()).unwrap();
        assert_eq!(
            out,
            "<head><meta name=\"repoview-mode\" content=\"static\" /><meta content=\"server\" name=\"x\"></head>"
        );
    }

    #[test]
    fn a_path_is_replaced_only_as_a_whole_path_on_both_sides() {
        let replace = |text: &str| replace_component(text, "/srv/p", ".", Left::Boundary);
        for kept in [
            "/srv/p.git",
            "/srv/p-2",
            "/srv/p_old",
            "/srv/px",
            "/mnt/srv/p",
            "x/srv/p",
            "/srv/pé",
        ] {
            assert_eq!(replace(kept), kept);
        }
        assert_eq!(replace("/srv/p"), ".");
        assert_eq!(
            replace("at /srv/p/a, '/srv/p' and \"/srv/p\"."),
            "at ./a, '.' and \".\"."
        );
        assert_eq!(replace("/srv/p:/srv/p/b"), ".:./b");
        // A rejected match does not hide a later one in the same text.
        assert_eq!(replace("/srv/p.git /srv/p"), "/srv/p.git .");
    }

    #[test]
    fn a_path_ending_a_sentence_is_still_a_whole_path() {
        let home = |text: &str| replace_component(text, "/srv/u", "~", Left::Boundary);
        assert_eq!(home("in /srv/u."), "in ~.");
        assert_eq!(home("in /srv/u.\nnext"), "in ~.\nnext");
        assert_eq!(home("(see /srv/u.)"), "(see ~.)");
        assert_eq!(home("in /srv/u. Then"), "in ~. Then");
        assert_eq!(home("in /srv/u..."), "in ~...");
        for kept in ["/srv/u.git", "/srv/u.next", "/srv/u..x", "/srv/u.old/a"] {
            assert_eq!(home(kept), kept);
        }
        let tool = |text: &str| replace_component(text, "/opt/bin/aep", "aep", Left::WholePath);
        assert_eq!(tool("ran /opt/bin/aep."), "ran aep.");
        assert_eq!(tool("ran /opt/bin/aep.\n"), "ran aep.\n");
        assert_eq!(tool("(/opt/bin/aep.)"), "(aep.)");
        assert_eq!(tool("/opt/bin/aep.sh"), "/opt/bin/aep.sh");
    }

    #[test]
    fn a_tool_path_ending_a_longer_path_takes_the_whole_path() {
        let replace = |text: &str| replace_component(text, "/bin/git", "git", Left::WholePath);
        assert_eq!(replace("/usr/bin/git and /bin/git"), "git and git");
        assert_eq!(replace("/bin/git-lfs /bin/gitx"), "/bin/git-lfs /bin/gitx");
        assert_eq!(replace("/bin/git: failed"), "git: failed");
    }

    #[test]
    fn trailing_slashes_are_stripped_but_root_stays_root() {
        assert_eq!(
            without_trailing_slashes(Path::new("a/b//")).as_os_str(),
            "a/b"
        );
        assert_eq!(without_trailing_slashes(Path::new("/")).as_os_str(), "/");
        assert_eq!(
            without_trailing_slashes(Path::new("site/")).as_os_str(),
            "site"
        );
    }

    #[test]
    fn requestable_refuses_what_the_client_refuses() {
        assert!(requestable("plan/artifacts/story:x"));
        assert!(!requestable("spec/roots/../ir"));
        assert!(!requestable("spec/roots/./ir"));
        assert!(!requestable("docs/"));
        assert!(!requestable("spec/roots//ir"));
    }
}
