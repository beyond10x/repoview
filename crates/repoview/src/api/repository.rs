//! The API routes the Repository page reads: `/api/vcs`, `/api/docs`, `/api/docs/{name}` and
//! `/api/tasks`.
//!
//! Every Git fact comes from `git` through [`repoview_sources::run`], which starts each
//! child with `GIT_OPTIONAL_LOCKS=0`, `core.fsmonitor=false` and `core.untrackedCache=false`, so
//! no request writes into `.git`. `/api/tasks` starts no process at all: it parses the Taskfile
//! as YAML, because `task` itself evaluates `vars: sh:` commands while listing.
//!
//! The project is the [`Env`] request extension: the server is built as
//! `router(state).layer(Extension(env))`. Without it these routes answer 500.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use axum::extract::Path as UrlPath;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum::{Extension, Router};
use repoview_sources::{Availability, Env, Outcome, TIMEOUT, run};
use serde_json::{Value, json};

use crate::server::AppState;

/// The top-level documents the page shows, in tab order.
pub const DOCUMENTS: [&str; 4] = ["README.md", "AGENTS.md", "STATUS.md", "CHANGELOG.md"];

/// The largest document `/api/docs/{name}` serves, in bytes.
pub const DOCUMENT_LIMIT: u64 = 1024 * 1024;

const COMMIT_LIMIT: usize = 50;
const TAG_LIMIT: usize = 30;

/// Name, the commit an annotated tag points at (else the tagged object), creator date.
const TAG_FORMAT: &str = concat!(
    "--format=%(refname:strip=2)%1f",
    "%(if)%(*objectname)%(then)%(*objectname)%(else)%(objectname)%(end)%1f",
    "%(creatordate:iso-strict)",
);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/vcs", get(vcs))
        .route("/api/docs", get(docs))
        .route("/api/docs/{name}", get(doc))
        .route("/api/tasks", get(tasks))
}

async fn vcs(Extension(env): Extension<Env>) -> Response {
    blocking(move || read_vcs(&env).map(Json)).await
}

async fn docs(Extension(env): Extension<Env>) -> Response {
    blocking(move || Ok::<_, Unavailable>(Json(list_documents(&env)))).await
}

async fn doc(Extension(env): Extension<Env>, UrlPath(name): UrlPath<String>) -> Response {
    blocking(move || read_document(&env, &name)).await
}

async fn tasks(Extension(env): Extension<Env>) -> Response {
    blocking(move || read_tasks(env.root()).map(Json)).await
}

/// Run `work` off the async runtime: every handler here starts subprocesses or reads files.
async fn blocking<T, E>(work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Response
where
    T: IntoResponse + Send + 'static,
    E: IntoResponse + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(body)) => body.into_response(),
        Ok(Err(error)) => error.into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "request failed\n").into_response(),
    }
}

/// Why a tool-backed route has no answer: the source shape (`availability`, `tool`, `tool_path`,
/// `diagnostic`) with 404 for `Absent` and 503 for `ToolMissing` and `Failed`.
#[derive(Debug)]
struct Unavailable {
    availability: Availability,
    tool: &'static str,
    tool_path: Option<String>,
    diagnostic: Option<String>,
}

impl Unavailable {
    fn absent(tool: &'static str, diagnostic: impl Into<String>) -> Self {
        Unavailable {
            availability: Availability::Absent,
            tool,
            tool_path: None,
            diagnostic: Some(diagnostic.into()),
        }
    }

    fn tool_missing(tool: &'static str) -> Self {
        Unavailable {
            availability: Availability::ToolMissing,
            tool,
            tool_path: None,
            diagnostic: Some(format!("{tool} not found on PATH")),
        }
    }

    /// `Failed` for a source read as a file rather than through a tool: no tool path.
    fn unreadable(tool: &'static str, diagnostic: impl Into<String>) -> Self {
        Unavailable {
            availability: Availability::Failed,
            tool,
            tool_path: None,
            diagnostic: Some(diagnostic.into()),
        }
    }

    fn failed(tool: &'static str, tool_path: &Path, diagnostic: impl Into<String>) -> Self {
        Unavailable {
            availability: Availability::Failed,
            tool,
            tool_path: Some(tool_path.to_string_lossy().into_owned()),
            diagnostic: Some(diagnostic.into()),
        }
    }
}

impl IntoResponse for Unavailable {
    fn into_response(self) -> Response {
        let status = match self.availability {
            Availability::Absent => StatusCode::NOT_FOUND,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = json!({
            "availability": self.availability.as_str(),
            "tool": self.tool,
            "tool_path": self.tool_path,
            "diagnostic": self.diagnostic,
        });
        (status, Json(body)).into_response()
    }
}

// ---- /api/vcs ------------------------------------------------------------------------------

/// `git` at a known path, run in the project.
struct Git<'a> {
    env: &'a Env,
    path: PathBuf,
}

impl Git<'_> {
    fn run(&self, args: &[&str]) -> Result<String, Unavailable> {
        match run(self.env, &self.path, args, TIMEOUT) {
            Outcome::Success { stdout } => Ok(stdout),
            Outcome::Failure { diagnostic } => {
                Err(Unavailable::failed("git", &self.path, diagnostic))
            }
        }
    }
}

fn read_vcs(env: &Env) -> Result<Value, Unavailable> {
    let path = env
        .find_tool("git")
        .ok_or_else(|| Unavailable::tool_missing("git"))?;
    if let Outcome::Failure { diagnostic } =
        run(env, &path, &["rev-parse", "--show-toplevel"], TIMEOUT)
    {
        return Err(Unavailable::absent("git", diagnostic));
    }
    let git = Git { env, path };
    let status = parse_status(&git.run(&[
        "status",
        "--porcelain=v2",
        "--branch",
        "-z",
        "--untracked-files=normal",
        "--renames",
    ])?);
    let commits = match status.head {
        Some(_) => parse_log(&git.run(&[
            "log",
            "-50",
            "--no-show-signature",
            "--format=%H%x00%aI%x00%an%x00%s",
            "HEAD",
            "--",
        ])?),
        None => Vec::new(),
    };
    // Status and commits are the route; a failing tags, remotes or worktrees read empties that
    // block and names its error, and the rest still answers.
    let (tags, tags_error) = block(
        git.run(&["tag", "--sort=-creatordate", TAG_FORMAT])
            .map(|out| parse_tags(&out)),
    );
    let (remotes, remotes_error) = block(git.run(&["remote", "-v"]).map(|out| parse_remotes(&out)));
    // `worktree list -z` needs Git 2.36; an older Git refuses `-z` (exit 129), and its
    // newline-separated porcelain is read instead.
    let worktree_list = git
        .run(&["worktree", "list", "--porcelain", "-z"])
        .map(|out| parse_worktrees(&out, '\0'))
        .or_else(|_| {
            git.run(&["worktree", "list", "--porcelain"])
                .map(|out| parse_worktrees(&out, '\n'))
        });
    let (worktrees, worktrees_error) = block(worktree_list);
    Ok(json!({
        "branch": status.branch,
        "head": status.head,
        "upstream": status.upstream,
        "ahead": status.ahead,
        "behind": status.behind,
        "dirty": status.dirty,
        "commits": commits,
        "tags": tags,
        "tags_error": tags_error,
        "remotes": remotes,
        "remotes_error": remotes_error,
        "worktrees": worktrees,
        "worktrees_error": worktrees_error,
    }))
}

/// One optional block of `/api/vcs`: its entries and `null`, or no entries and the diagnostic.
fn block(read: Result<Vec<Value>, Unavailable>) -> (Vec<Value>, Option<String>) {
    match read {
        Ok(entries) => (entries, None),
        Err(error) => (Vec::new(), error.diagnostic),
    }
}

#[derive(Debug, Default, PartialEq)]
struct Status {
    branch: Option<String>,
    head: Option<String>,
    upstream: Option<String>,
    ahead: Option<u64>,
    behind: Option<u64>,
    dirty: Vec<Value>,
}

/// `git status --porcelain=v2 --branch -z`: `# branch.*` headers, then one NUL-terminated entry
/// per path; a rename or copy entry (`2`) is followed by its original path as a separate field.
fn parse_status(output: &str) -> Status {
    let mut status = Status::default();
    let mut fields = output.split('\0');
    while let Some(field) = fields.next() {
        if let Some(header) = field.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.oid" if value != "(initial)" => status.head = Some(value.to_owned()),
                "branch.head" if value != "(detached)" => status.branch = Some(value.to_owned()),
                "branch.upstream" => status.upstream = Some(value.to_owned()),
                "branch.ab" => {
                    let mut counts = value.split(' ');
                    status.ahead = counts
                        .next()
                        .and_then(|n| n.trim_start_matches('+').parse().ok());
                    status.behind = counts
                        .next()
                        .and_then(|n| n.trim_start_matches('-').parse().ok());
                }
                _ => {}
            }
            continue;
        }
        let entry = match field.split_once(' ') {
            Some(("1", rest)) => nth_rest(rest, 7).map(|(xy, path)| (xy, path, None)),
            Some(("2", rest)) => {
                nth_rest(rest, 8).map(|(xy, path)| (xy, path, fields.next().map(str::to_owned)))
            }
            Some(("u", rest)) => nth_rest(rest, 9).map(|(xy, path)| (xy, path, None)),
            Some(("?", path)) => Some(("??", path, None)),
            _ => None,
        };
        if let Some((xy, path, orig_path)) = entry {
            status
                .dirty
                .push(json!({ "path": path, "status": xy, "orig_path": orig_path }));
        }
    }
    status
}

/// The first space-separated word of `rest` (the `XY` status) and what follows its `skip`
/// space-separated fields: the path, which may itself contain spaces.
fn nth_rest(rest: &str, skip: usize) -> Option<(&str, &str)> {
    let mut parts = rest.splitn(skip + 1, ' ');
    let xy = parts.next()?;
    parts.nth(skip - 1).map(|path| (xy, path))
}

/// `git log --format=%H%x00%aI%x00%an%x00%s`: one commit per line, fields separated by NUL. Git
/// keeps NUL and newlines out of an author name, and `%s` joins the subject onto one line, so
/// neither separator can occur inside a field the split relies on; the subject comes last and
/// keeps anything else.
fn parse_log(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let mut fields = record.splitn(4, '\0');
            let sha = fields.next()?;
            let date = fields.next()?;
            let author = fields.next()?;
            let subject = fields.next().unwrap_or("");
            Some(json!({ "sha": sha, "author": author, "date": date, "subject": subject }))
        })
        .take(COMMIT_LIMIT)
        .collect()
}

/// `git tag --format=<name>%1f<commit>%1f<date>`, newest first already; the first [`TAG_LIMIT`].
fn parse_tags(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\u{1f}');
            let name = fields.next()?;
            let sha = fields.next()?;
            let date = fields.next()?;
            Some(json!({ "name": name, "sha": sha, "date": date }))
        })
        .take(TAG_LIMIT)
        .collect()
}

/// `git remote -v`: one entry per remote with its fetch URL, userinfo removed.
fn parse_remotes(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter_map(|line| {
            let (name, url) = line.split_once('\t')?;
            let url = url.strip_suffix(" (fetch)")?;
            Some(json!({ "name": name, "url": redact_url(url) }))
        })
        .collect()
}

/// `url` without credentials. A URL with a scheme loses its whole userinfo (`user:password@` or a
/// token used as the user name); an scp-like `user@host:path` keeps a plain user name and loses a
/// `user:password@`. Anything else is returned unchanged.
pub fn redact_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        return match authority.rfind('@') {
            Some(at) => format!("{scheme}://{}{tail}", &authority[at + 1..]),
            None => url.to_owned(),
        };
    }
    let host_part = &url[..url.find('/').unwrap_or(url.len())];
    match host_part.rfind('@') {
        Some(at) if host_part[..at].contains(':') => url[at + 1..].to_owned(),
        _ => url.to_owned(),
    }
}

/// `git worktree list --porcelain`: attributes ended by `terminator` (NUL with `-z`, else a
/// newline), an empty attribute between records.
fn parse_worktrees(output: &str, terminator: char) -> Vec<Value> {
    let mut worktrees = Vec::new();
    let mut current: Option<serde_json::Map<String, Value>> = None;
    for attribute in output.split(terminator) {
        let (key, value) = attribute.split_once(' ').unwrap_or((attribute, ""));
        match key {
            "worktree" => {
                worktrees.extend(current.take().map(Value::Object));
                let mut record = serde_json::Map::new();
                record.insert("path".into(), json!(value));
                record.insert("head".into(), Value::Null);
                record.insert("branch".into(), Value::Null);
                record.insert("locked".into(), json!(false));
                record.insert("prunable".into(), json!(false));
                current = Some(record);
            }
            _ => {
                let Some(record) = current.as_mut() else {
                    continue;
                };
                match key {
                    "HEAD" if !value.bytes().all(|b| b == b'0') => {
                        record.insert("head".into(), json!(value));
                    }
                    "branch" => {
                        let branch = value.strip_prefix("refs/heads/").unwrap_or(value);
                        record.insert("branch".into(), json!(branch));
                    }
                    "locked" => {
                        record.insert("locked".into(), json!(true));
                    }
                    "prunable" => {
                        record.insert("prunable".into(), json!(true));
                    }
                    _ => {}
                }
            }
        }
    }
    worktrees.extend(current.map(Value::Object));
    worktrees
}

// ---- /api/docs -----------------------------------------------------------------------------

/// The Git directory of the project (`git rev-parse --absolute-git-dir`), canonical; `None`
/// outside Git or without `git` on `PATH`.
fn git_dir(env: &Env) -> Option<PathBuf> {
    let git = env.find_tool("git")?;
    match run(env, &git, &["rev-parse", "--absolute-git-dir"], TIMEOUT) {
        Outcome::Success { stdout } => std::fs::canonicalize(stdout.trim_end_matches('\n')).ok(),
        Outcome::Failure { .. } => None,
    }
}

/// Document `name` opened for reading, if it is a regular file directly in the project root and
/// the root is not inside the Git directory. A symlink is refused wherever it points (opened with
/// `O_NOFOLLOW`, so it cannot be swapped in after a check), and so is anything that is not a
/// regular file (`O_NONBLOCK` keeps a FIFO from blocking the open). `git_dir` is [`git_dir`].
fn open_document(env: &Env, name: &str, git_dir: Option<&Path>) -> Option<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    if !DOCUMENTS.contains(&name) {
        return None;
    }
    if let Some(git_dir) = git_dir {
        let root = std::fs::canonicalize(env.root()).ok()?;
        if root.starts_with(git_dir) {
            return None;
        }
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(env.root().join(name))
        .ok()?;
    file.metadata().ok()?.is_file().then_some(file)
}

fn list_documents(env: &Env) -> Value {
    let git_dir = git_dir(env);
    DOCUMENTS
        .iter()
        .map(|name| {
            let present = open_document(env, name, git_dir.as_deref()).is_some();
            json!({ "name": name, "present": present })
        })
        .collect()
}

/// Why `/api/docs/{name}` has no document: not one of [`DOCUMENTS`], not a regular file in the
/// project, unreadable (404), or over [`DOCUMENT_LIMIT`] (413).
#[derive(Debug)]
enum DocumentError {
    NotFound,
    TooLarge,
}

impl IntoResponse for DocumentError {
    fn into_response(self) -> Response {
        match self {
            DocumentError::NotFound => (StatusCode::NOT_FOUND, "not found\n").into_response(),
            DocumentError::TooLarge => {
                let body = json!({ "limit": DOCUMENT_LIMIT });
                (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response()
            }
        }
    }
}

fn read_document(env: &Env, name: &str) -> Result<Json<Value>, DocumentError> {
    if !DOCUMENTS.contains(&name) {
        return Err(DocumentError::NotFound);
    }
    let file = open_document(env, name, git_dir(env).as_deref()).ok_or(DocumentError::NotFound)?;
    let size = file.metadata().map_err(|_| DocumentError::NotFound)?.len();
    if size > DOCUMENT_LIMIT {
        return Err(DocumentError::TooLarge);
    }
    // The file may grow between the size check and the read; read one byte past the limit.
    let mut bytes = Vec::new();
    file.take(DOCUMENT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DocumentError::NotFound)?;
    if bytes.len() as u64 > DOCUMENT_LIMIT {
        return Err(DocumentError::TooLarge);
    }
    let markdown = String::from_utf8_lossy(&bytes);
    Ok(Json(json!({ "name": name, "markdown": markdown })))
}

// ---- /api/tasks ----------------------------------------------------------------------------

/// The largest Taskfile `/api/tasks` reads, in bytes; a larger root is `Failed`, a larger include
/// is refused.
pub const TASKFILE_LIMIT: u64 = 1024 * 1024;

/// How deep `includes:` nest: the root's includes are depth 1.
pub const INCLUDE_DEPTH: usize = 4;

/// How many Taskfiles one request reads, the root included; later includes are refused.
pub const TASKFILE_BUDGET: usize = 128;

/// How many YAML nodes one request builds over all its Taskfiles, every alias counted at each
/// place it is used. serde_yaml_ng's own limit counts alias jumps, not the nodes they copy.
pub const NODE_LIMIT: usize = 200_000;

/// How deep flow (`[`, `{`) or block nesting may go before a Taskfile is parsed at all; libyaml's
/// scanner is quadratic in flow depth and serde_yaml_ng refuses depth only after the whole scan.
pub const NESTING_LIMIT: usize = 128;

/// How many tasks and refused includes one response carries; past it `truncated` is `true`.
pub const ENTRY_LIMIT: usize = 10_000;

/// A parsed Taskfile, shared by every include of the same file.
type Taskfile = std::rc::Rc<serde_yaml_ng::Mapping>;

/// The names `task` looks for in a directory, in its order.
const TASKFILE_NAMES: [&str; 8] = [
    "Taskfile.yml",
    "taskfile.yml",
    "Taskfile.yaml",
    "taskfile.yaml",
    "Taskfile.dist.yml",
    "taskfile.dist.yml",
    "Taskfile.dist.yaml",
    "taskfile.dist.yaml",
];

/// Why one Taskfile was not read.
enum TaskfileError {
    /// Nothing there, or not a regular file: the next default name is tried.
    Missing,
    /// There, but refused or unreadable, with the reason.
    Refused(String),
}

/// One include being read: where it sits in the namespace tree and what it passes down.
struct Include<'a> {
    /// `a:b:` for tasks of `b` included by `a`; empty at the root and under `flatten`.
    prefix: String,
    /// The include's own name, `a:b`, for the `refused` list.
    namespace: String,
    internal: bool,
    excludes: Vec<String>,
    /// Canonical paths of the Taskfiles above this one.
    ancestors: &'a [PathBuf],
    depth: usize,
}

/// The tasks and refused includes collected over one request.
struct TaskReader {
    root: PathBuf,
    /// Taskfiles still to be read, of [`TASKFILE_BUDGET`].
    budget: usize,
    /// YAML nodes still to be built, of [`NODE_LIMIT`].
    nodes: usize,
    /// Each canonical file read once: its Taskfile, or why it was refused.
    parsed: HashMap<PathBuf, Result<Taskfile, String>>,
    tasks: Vec<Value>,
    refused: Vec<Value>,
    /// [`ENTRY_LIMIT`] was reached and later tasks and refusals were dropped.
    truncated: bool,
}

fn read_tasks(project: &Path) -> Result<Value, Unavailable> {
    const SOURCE: &str = "Taskfile.yml";
    let root = std::fs::canonicalize(project)
        .map_err(|error| Unavailable::unreadable(SOURCE, format!("project: {error}")))?;
    let mut reader = TaskReader {
        root,
        budget: TASKFILE_BUDGET,
        nodes: NODE_LIMIT,
        parsed: HashMap::new(),
        truncated: false,
        tasks: Vec::new(),
        refused: Vec::new(),
    };
    for name in TASKFILE_NAMES {
        let (canonical, taskfile) = match reader.load(&reader.root.join(name)) {
            Ok(loaded) => loaded,
            Err(TaskfileError::Missing) => continue,
            Err(TaskfileError::Refused(reason)) => {
                return Err(Unavailable::unreadable(SOURCE, format!("{name}: {reason}")));
            }
        };
        let top = Include {
            prefix: String::new(),
            namespace: String::new(),
            internal: false,
            excludes: Vec::new(),
            ancestors: &[],
            depth: 0,
        };
        reader.collect(&canonical, &taskfile, &top);
        return Ok(json!({
            "file": name,
            "tasks": reader.tasks,
            "refused": reader.refused,
            "truncated": reader.truncated,
        }));
    }
    Err(Unavailable::absent(SOURCE, "no Taskfile in the project"))
}

/// A YAML scalar key as `task` reads it: strings, numbers and booleans by their text.
fn yaml_key(key: &serde_yaml_ng::Value) -> Option<String> {
    match key {
        serde_yaml_ng::Value::String(text) => Some(text.clone()),
        serde_yaml_ng::Value::Number(number) => Some(number.to_string()),
        serde_yaml_ng::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The strings of a YAML sequence; anything else is no strings.
fn yaml_strings(value: Option<&serde_yaml_ng::Value>) -> Vec<String> {
    value
        .and_then(serde_yaml_ng::Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Why an include path is refused before anything is resolved: it is not a plain relative path
/// that stays below the including Taskfile's directory.
fn include_path_refusal(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        return Some("empty path");
    }
    if path.contains("{{") {
        return Some("templated path; templates are not evaluated");
    }
    let first = path.split('/').next().unwrap_or("");
    if path.contains("://") || first.contains(':') {
        return Some("remote Taskfile; only files inside the project are read");
    }
    if path.starts_with('/') {
        return Some("absolute path; only files inside the project are read");
    }
    if path.starts_with('~') {
        return Some("home directory path; only files inside the project are read");
    }
    if Path::new(path)
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return Some("path contains `..`");
    }
    None
}

impl TaskReader {
    /// The Taskfile at `path`, canonical and parsed, if it resolves to a regular file inside the
    /// project and outside `.git`, is at most [`TASKFILE_LIMIT`] bytes and is a YAML mapping.
    /// Opened with `O_NOFOLLOW` (the canonical path has no symlink left) and `O_NONBLOCK` (a FIFO
    /// does not block). Counts against [`TASKFILE_BUDGET`] once opened.
    fn load(&mut self, path: &Path) -> Result<(PathBuf, Taskfile), TaskfileError> {
        let canonical = self.resolve(path)?;
        self.read(&canonical).map(|taskfile| (canonical, taskfile))
    }

    /// `path` with every symlink resolved, if that stays inside the project and outside `.git`.
    fn resolve(&self, path: &Path) -> Result<PathBuf, TaskfileError> {
        let canonical = match std::fs::canonicalize(path) {
            Ok(canonical) => canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(TaskfileError::Missing);
            }
            Err(error) => return Err(TaskfileError::Refused(error.to_string())),
        };
        let Ok(inside) = canonical.strip_prefix(&self.root) else {
            return Err(TaskfileError::Refused(
                "resolves outside the project".to_owned(),
            ));
        };
        if inside.components().any(|part| part.as_os_str() == ".git") {
            return Err(TaskfileError::Refused("resolves into `.git`".to_owned()));
        }
        Ok(canonical)
    }

    /// The Taskfile at `canonical`, parsed on its first read and shared after that.
    fn read(&mut self, canonical: &Path) -> Result<Taskfile, TaskfileError> {
        if canonical.is_dir() {
            return Err(TaskfileError::Missing);
        }
        if let Some(parsed) = self.parsed.get(canonical) {
            return parsed.clone().map_err(TaskfileError::Refused);
        }
        let parsed = match self.read_new(canonical) {
            Ok(taskfile) => Ok(Taskfile::new(taskfile)),
            Err(TaskfileError::Refused(reason)) => Err(reason),
            Err(TaskfileError::Missing) => return Err(TaskfileError::Missing),
        };
        self.parsed.insert(canonical.to_owned(), parsed.clone());
        parsed.map_err(TaskfileError::Refused)
    }

    fn read_new(&mut self, canonical: &Path) -> Result<serde_yaml_ng::Mapping, TaskfileError> {
        use std::os::unix::fs::OpenOptionsExt;
        if self.budget == 0 {
            return Err(TaskfileError::Refused(format!(
                "include limit of {TASKFILE_BUDGET} Taskfiles reached"
            )));
        }
        let refused = |error: std::io::Error| TaskfileError::Refused(error.to_string());
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(canonical)
            .map_err(refused)?;
        if !file.metadata().map_err(refused)?.is_file() {
            return Err(TaskfileError::Missing);
        }
        self.budget -= 1;
        let mut bytes = Vec::new();
        file.take(TASKFILE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(refused)?;
        if bytes.len() as u64 > TASKFILE_LIMIT {
            return Err(TaskfileError::Refused(
                "larger than 1 MiB; not read".to_owned(),
            ));
        }
        if nesting_exceeds(&bytes, NESTING_LIMIT) {
            return Err(TaskfileError::Refused(format!(
                "nested more than {NESTING_LIMIT} levels deep; not parsed"
            )));
        }
        let mut value = parse_counted(&bytes, &mut self.nodes).map_err(|error| {
            TaskfileError::Refused(match error {
                ParseError::Nodes => {
                    format!("more than {NODE_LIMIT} YAML nodes with aliases expanded; not read")
                }
                ParseError::Yaml(error) => format!("invalid YAML: {error}"),
            })
        })?;
        value
            .apply_merge()
            .map_err(|error| TaskfileError::Refused(format!("invalid merge key: {error}")))?;
        match value {
            serde_yaml_ng::Value::Mapping(taskfile) => Ok(taskfile),
            // An empty file is a Taskfile without tasks.
            serde_yaml_ng::Value::Null => Ok(serde_yaml_ng::Mapping::new()),
            _ => Err(TaskfileError::Refused(
                "not a YAML mapping; not a Taskfile".to_owned(),
            )),
        }
    }

    /// The tasks of `taskfile` (at canonical `path`), then those of its includes, depth first.
    fn collect(&mut self, path: &Path, taskfile: &serde_yaml_ng::Mapping, at: &Include) {
        if let Some(tasks) = taskfile.get("tasks").and_then(|v| v.as_mapping()) {
            for (key, task) in tasks {
                let Some(name) = yaml_key(key) else { continue };
                if at.excludes.contains(&name) {
                    continue;
                }
                if !self.room() {
                    return;
                }
                self.tasks
                    .push(task_entry(&at.prefix, &name, task, at.internal));
            }
        }
        let Some(includes) = taskfile.get("includes").and_then(|v| v.as_mapping()) else {
            return;
        };
        let mut ancestors = at.ancestors.to_vec();
        ancestors.push(path.to_owned());
        let dir = path.parent().unwrap_or(&self.root).to_owned();
        for (key, include) in includes {
            if !self.room() {
                return;
            }
            let Some(name) = yaml_key(key) else { continue };
            let namespace = if at.namespace.is_empty() {
                name.clone()
            } else {
                format!("{}:{name}", at.namespace)
            };
            let (target, options) = match include {
                serde_yaml_ng::Value::String(target) => (Some(target.as_str()), None),
                serde_yaml_ng::Value::Mapping(options) => (
                    options.get("taskfile").and_then(|v| v.as_str()),
                    Some(options),
                ),
                _ => (None, None),
            };
            let flag = |field: &str| {
                options
                    .and_then(|o| o.get(field))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            };
            let Some(target) = target else {
                self.refuse(&namespace, None, "no `taskfile` path");
                continue;
            };
            let refusal = if at.depth + 1 > INCLUDE_DEPTH {
                Err(format!("include depth over {INCLUDE_DEPTH}"))
            } else if let Some(reason) = include_path_refusal(target) {
                Err(reason.to_owned())
            } else {
                match self.load_include(&dir.join(target), &ancestors) {
                    Ok(loaded) => Ok(loaded),
                    Err(TaskfileError::Missing) if flag("optional") => continue,
                    Err(TaskfileError::Missing) => Err("not found".to_owned()),
                    Err(TaskfileError::Refused(reason)) => Err(reason),
                }
            };
            let (canonical, child) = match refusal {
                Ok(loaded) => loaded,
                Err(reason) => {
                    self.refuse(&namespace, Some(target), &reason);
                    continue;
                }
            };
            let below = Include {
                prefix: if flag("flatten") {
                    at.prefix.clone()
                } else {
                    format!("{}{name}:", at.prefix)
                },
                namespace,
                internal: at.internal || flag("internal"),
                excludes: yaml_strings(options.and_then(|o| o.get("excludes"))),
                ancestors: &ancestors,
                depth: at.depth + 1,
            };
            self.collect(&canonical, &child, &below);
        }
    }

    /// An included Taskfile: `path` itself, or the first of [`TASKFILE_NAMES`] in it when it is a
    /// directory. A Taskfile already among `ancestors` is a cycle and is not read again.
    fn load_include(
        &mut self,
        path: &Path,
        ancestors: &[PathBuf],
    ) -> Result<(PathBuf, Taskfile), TaskfileError> {
        let resolved = self.resolve(path)?;
        let candidates: Vec<PathBuf> = if resolved.is_dir() {
            TASKFILE_NAMES
                .iter()
                .map(|name| resolved.join(name))
                .collect()
        } else {
            vec![resolved]
        };
        for candidate in candidates {
            let canonical = match self.resolve(&candidate) {
                Ok(canonical) => canonical,
                Err(TaskfileError::Missing) => continue,
                Err(refused) => return Err(refused),
            };
            if ancestors.contains(&canonical) {
                return Err(TaskfileError::Refused("include cycle".to_owned()));
            }
            match self.read(&canonical) {
                Err(TaskfileError::Missing) => continue,
                read => return read.map(|taskfile| (canonical, taskfile)),
            }
        }
        Err(TaskfileError::Missing)
    }

    /// Whether one more task or refusal fits under [`ENTRY_LIMIT`]; marks the response truncated
    /// when it does not.
    fn room(&mut self) -> bool {
        if self.tasks.len() + self.refused.len() < ENTRY_LIMIT {
            return true;
        }
        self.truncated = true;
        false
    }

    fn refuse(&mut self, namespace: &str, target: Option<&str>, reason: &str) {
        if !self.room() {
            return;
        }
        self.refused.push(json!({
            "include": namespace,
            "taskfile": target,
            "reason": reason,
        }));
    }
}

/// Whether `bytes` nest deeper than `limit` before any parsing: flow brackets (`[`, `{`) open
/// anywhere on a line, and block levels are the stack of strictly growing indentations plus the
/// compact `- ` / `? ` entries opening a line. Quoting is not tracked, so brackets inside strings
/// count too; a real Taskfile does not hold 128 unclosed ones. Linear in the length.
fn nesting_exceeds(bytes: &[u8], limit: usize) -> bool {
    let mut flow = 0usize;
    let mut indents: Vec<usize> = Vec::new();
    for line in bytes.split(|&b| b == b'\n') {
        let indent = line.iter().take_while(|&&b| b == b' ').count();
        let mut rest = &line[indent..];
        if flow == 0 && !rest.is_empty() && rest[0] != b'#' && rest != b"\r" {
            while indents.last().is_some_and(|&top| top >= indent) {
                indents.pop();
            }
            indents.push(indent);
            let mut compact = 0;
            while let [b'-' | b'?', b' ' | b'\t', tail @ ..] = rest {
                compact += 1;
                rest = tail;
                let spaces = rest
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                rest = &rest[spaces..];
            }
            if indents.len() + compact > limit {
                return true;
            }
        }
        for &byte in line {
            match byte {
                b'[' | b'{' => {
                    flow += 1;
                    if flow > limit {
                        return true;
                    }
                }
                b']' | b'}' => flow = flow.saturating_sub(1),
                _ => {}
            }
        }
    }
    false
}

/// Why [`parse_counted`] built no value.
enum ParseError {
    /// The node budget ran out.
    Nodes,
    Yaml(serde_yaml_ng::Error),
}

/// `bytes` as a YAML value, built node by node while `budget` is spent: each scalar, sequence and
/// mapping costs one, at every place an alias repeats it, and building stops as soon as the budget
/// is gone. Duplicate mapping keys are an error, as in serde_yaml_ng's own `Value`.
fn parse_counted(bytes: &[u8], budget: &mut usize) -> Result<serde_yaml_ng::Value, ParseError> {
    use serde::de::DeserializeSeed;
    let exhausted = std::cell::Cell::new(false);
    let seed = CountedValue {
        budget: std::cell::Cell::new(*budget),
        exhausted: &exhausted,
    };
    let deserializer = serde_yaml_ng::Deserializer::from_slice(bytes);
    let result = (&seed).deserialize(deserializer);
    *budget = seed.budget.get();
    match result {
        Ok(value) => Ok(value),
        Err(_) if exhausted.get() => Err(ParseError::Nodes),
        Err(error) => Err(ParseError::Yaml(error)),
    }
}

/// The seed [`parse_counted`] builds with.
struct CountedValue<'a> {
    budget: std::cell::Cell<usize>,
    exhausted: &'a std::cell::Cell<bool>,
}

impl CountedValue<'_> {
    fn spend<E: serde::de::Error>(&self) -> Result<(), E> {
        match self.budget.get().checked_sub(1) {
            Some(left) => {
                self.budget.set(left);
                Ok(())
            }
            None => {
                self.exhausted.set(true);
                Err(E::custom("YAML node limit reached"))
            }
        }
    }
}

impl<'de> serde::de::DeserializeSeed<'de> for &CountedValue<'_> {
    type Value = serde_yaml_ng::Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for &CountedValue<'_> {
    type Value = serde_yaml_ng::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any YAML value")
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::Bool(value))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::Number(value.into()))
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::Number(value.into()))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::String(value.to_owned()))
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::String(value))
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.spend()?;
        Ok(serde_yaml_ng::Value::Null)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.visit_unit()
    }

    fn visit_some<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        serde::de::DeserializeSeed::deserialize(self, deserializer)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut items: A,
    ) -> Result<Self::Value, A::Error> {
        self.spend()?;
        let mut sequence = Vec::new();
        while let Some(item) = items.next_element_seed(self)? {
            sequence.push(item);
        }
        Ok(serde_yaml_ng::Value::Sequence(sequence))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut entries: A,
    ) -> Result<Self::Value, A::Error> {
        use serde::de::Error;
        self.spend()?;
        let mut mapping = serde_yaml_ng::Mapping::new();
        while let Some(key) = entries.next_key_seed(self)? {
            if mapping.contains_key(&key) {
                return Err(A::Error::custom("duplicate mapping key"));
            }
            let value = entries.next_value_seed(self)?;
            mapping.insert(key, value);
        }
        Ok(serde_yaml_ng::Value::Mapping(mapping))
    }

    fn visit_enum<A: serde::de::EnumAccess<'de>>(self, tagged: A) -> Result<Self::Value, A::Error> {
        use serde::de::VariantAccess;
        self.spend()?;
        let (tag, contents) = tagged.variant::<String>()?;
        let value = contents.newtype_variant_seed(self)?;
        Ok(serde_yaml_ng::Value::Tagged(Box::new(
            serde_yaml_ng::value::TaggedValue {
                tag: serde_yaml_ng::value::Tag::new(tag),
                value,
            },
        )))
    }
}

/// One task as the page shows it. `internal` is the task's own flag or its include's.
fn task_entry(prefix: &str, name: &str, task: &serde_yaml_ng::Value, internal: bool) -> Value {
    let field = |key: &str| task.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    let own_internal = task
        .get("internal")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let aliases: Vec<String> = yaml_strings(task.get("aliases"))
        .into_iter()
        .map(|alias| format!("{prefix}{alias}"))
        .collect();
    json!({
        "name": format!("{prefix}{name}"),
        "desc": field("desc"),
        "summary": field("summary"),
        "internal": internal || own_internal,
        "aliases": aliases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_url_removes_credentials_and_keeps_the_rest() {
        for (url, expected) in [
            (
                "https://x-access-token:abc@github.com/o/r.git",
                "https://github.com/o/r.git",
            ),
            ("https://token@github.com", "https://github.com"),
            ("https://a:b@c@host/p?x=@y#@z", "https://host/p?x=@y#@z"),
            ("https://github.com/o/r.git", "https://github.com/o/r.git"),
            ("git@github.com:o/r.git", "git@github.com:o/r.git"),
            ("u:p@host:o/r.git", "host:o/r.git"),
            ("/srv/a@b/r.git", "/srv/a@b/r.git"),
            ("../sibling", "../sibling"),
        ] {
            assert_eq!(redact_url(url), expected, "{url}");
        }
    }

    #[test]
    fn status_without_entries_or_headers_is_empty() {
        assert_eq!(parse_status(""), Status::default());
    }

    #[test]
    fn worktrees_without_a_worktree_line_are_ignored() {
        assert_eq!(parse_worktrees("HEAD abc\0\0", '\0'), Vec::<Value>::new());
        assert_eq!(parse_worktrees("HEAD abc\n\n", '\n'), Vec::<Value>::new());
    }

    /// Correction round 1: the fields are NUL-separated, so U+001F and other control characters
    /// in an author or subject stay where they are.
    #[test]
    fn log_fields_split_on_nul_only() {
        let output = "abc\u{0}2026-10-05T16:00:00+02:00\0Eve\u{1f}Forged\0a\u{1f}b\0c\n\
                      def\u{0}2026-10-04T09:00:00+02:00\0Ada\0second\n";
        assert_eq!(
            parse_log(output),
            [
                json!({
                    "sha": "abc",
                    "author": "Eve\u{1f}Forged",
                    "date": "2026-10-05T16:00:00+02:00",
                    "subject": "a\u{1f}b\0c",
                }),
                json!({
                    "sha": "def",
                    "author": "Ada",
                    "date": "2026-10-04T09:00:00+02:00",
                    "subject": "second",
                }),
            ]
        );
    }
}
