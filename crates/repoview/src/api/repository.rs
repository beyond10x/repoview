//! The API routes the Repository page reads: `/api/vcs`, `/api/docs` and `/api/docs/{name}`.
//!
//! Every Git fact comes from `git` through [`repoview_sources::run`], which starts each
//! child with `GIT_OPTIONAL_LOCKS=0`, `core.fsmonitor=false` and `core.untrackedCache=false`, so
//! no request writes into `.git`.
//!
//! The project is the [`Env`] request extension: the server is built as
//! `router(state).layer(Extension(env))`. Without it these routes answer 500.

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
