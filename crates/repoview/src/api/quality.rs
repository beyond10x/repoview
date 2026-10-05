//! `GET /api/quality`: Codegate assessments per language, or why there is none.
//!
//! Languages are detected from marker files at the project root; which of them Codegate supports
//! is read from `codegate capabilities`, never from a list here. The first request starts one
//! `codegate assess` per supported language in the background and answers at once; each language
//! is `running` until its assessment finishes, fails or times out. Later requests read the same
//! run. The project arrives as the `Extension<Env>` that `repoview open` layers over the router.
//!
//! Every `codegate` child leads its own process group and is registered while it runs, so
//! [`shutdown`] (also installed as a process-exit hook) kills whatever is still running when the
//! server stops.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, Once, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum::{Extension, Router};
use repoview_sources::{Env, Outcome, TIMEOUT, run, truncate_diagnostic};
use serde::Serialize;
use serde_json::{Value, json};

use crate::server::AppState;

/// The assessing tool, as it is looked up on `PATH` and named on the wire.
pub const TOOL: &str = "codegate";

/// How long one `codegate assess` may run.
pub const ASSESS_TIMEOUT: Duration = Duration::from_secs(120);

/// Marker files at the project root, in wire order. `build.gradle*` is matched as a prefix.
const MARKERS: [(&str, &str); 5] = [
    ("go.mod", "go"),
    ("Cargo.toml", "rust"),
    ("package.json", "typescript"),
    ("pom.xml", "java"),
    ("build.gradle", "java"),
];

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/quality", get(quality))
}

/// The languages of the project at `env.root`, each once, in the order of [`MARKERS`], then
/// `markdown` when Git tracks any `*.md` file.
pub fn detect_languages(env: &Env) -> Vec<&'static str> {
    let names: Vec<String> = std::fs::read_dir(env.root())
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let mut languages = Vec::new();
    for (marker, language) in MARKERS {
        let present = names.iter().any(|name| {
            name == marker || (marker == "build.gradle" && name.starts_with("build.gradle"))
        });
        if present && !languages.contains(&language) {
            languages.push(language);
        }
    }
    if tracks_markdown(env) {
        languages.push("markdown");
    }
    languages
}

fn tracks_markdown(env: &Env) -> bool {
    let Some(git) = env.find_tool("git") else {
        return false;
    };
    matches!(
        run(env, &git, &["ls-files", "-z", "--", "*.md"], TIMEOUT),
        Outcome::Success { stdout } if !stdout.is_empty()
    )
}

/// The `language` field of each entry of `codegate capabilities` output. An entry without a
/// string `language` is skipped; output that is not a JSON array is an error.
pub fn parse_capabilities(stdout: &str) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_str(stdout)
        .map_err(|error| format!("codegate capabilities printed no JSON document: {error}"))?;
    let Value::Array(entries) = value else {
        return Err("codegate capabilities printed no JSON array".to_owned());
    };
    Ok(entries
        .iter()
        .filter_map(|entry| entry.get("language")?.as_str().map(str::to_owned))
        .collect())
}

/// `repoview.quality` status of one language on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Status {
    Assessed,
    NotAssessed,
    Failed,
    Running,
}

#[derive(Debug, Clone, Serialize)]
struct LanguageEntry {
    language: String,
    status: Status,
    reason: Option<String>,
    stderr: Option<String>,
    assessment: Option<Value>,
}

impl LanguageEntry {
    fn new(language: &str, status: Status) -> LanguageEntry {
        LanguageEntry {
            language: language.to_owned(),
            status,
            reason: None,
            stderr: None,
            assessment: None,
        }
    }

    fn failed(language: &str, reason: String, stderr: String) -> LanguageEntry {
        LanguageEntry {
            reason: Some(reason),
            stderr: Some(stderr),
            ..LanguageEntry::new(language, Status::Failed)
        }
    }
}

/// One run of every assessment for a project: started once, read by every later request.
pub struct Assessments {
    tool_path: PathBuf,
    languages: Mutex<Vec<LanguageEntry>>,
}

impl Assessments {
    /// Detect the languages, read `codegate capabilities`, and start one `assess` per supported
    /// language on its own thread, each bounded by `timeout`. Returns once they are started.
    pub fn start(env: Env, tool_path: PathBuf, timeout: Duration) -> Arc<Assessments> {
        let detected = detect_languages(&env);
        let supported = match run_codegate(&env, &tool_path, &["capabilities"], TIMEOUT) {
            Run::Success(stdout) => parse_capabilities(&stdout),
            Run::Failure(diagnostic) | Run::TimedOut(diagnostic) => Err(diagnostic),
        };
        let entries = detected
            .iter()
            .map(|&language| match &supported {
                Err(diagnostic) => LanguageEntry::failed(
                    language,
                    format!("{TOOL} capabilities failed"),
                    diagnostic.clone(),
                ),
                Ok(supported) if supported.iter().any(|name| name == language) => {
                    LanguageEntry::new(language, Status::Running)
                }
                Ok(_) => LanguageEntry {
                    reason: Some(format!("{TOOL} does not support {language}")),
                    ..LanguageEntry::new(language, Status::NotAssessed)
                },
            })
            .collect::<Vec<_>>();
        let assessments = Arc::new(Assessments {
            tool_path,
            languages: Mutex::new(entries.clone()),
        });
        let env = Arc::new(env);
        for (index, entry) in entries.iter().enumerate() {
            if entry.status != Status::Running {
                continue;
            }
            let language = entry.language.clone();
            let env = Arc::clone(&env);
            let assessments = Arc::clone(&assessments);
            std::thread::spawn(move || {
                let finished = assess(&env, &assessments.tool_path, &language, timeout);
                assessments.lock()[index] = finished;
            });
        }
        assessments
    }

    /// The `/api/quality` document as it stands now.
    pub fn document(&self) -> Value {
        json!({
            "tool": TOOL,
            "tool_path": self.tool_path.to_string_lossy(),
            "languages": *self.lock(),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Vec<LanguageEntry>> {
        self.languages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// `codegate --root <root> --language <language> --format json assess --gate all`, its JSON
/// passed through unchanged.
fn assess(env: &Env, tool_path: &Path, language: &str, timeout: Duration) -> LanguageEntry {
    let root = env.root().to_string_lossy();
    let args = [
        "--root",
        &root,
        "--language",
        language,
        "--format",
        "json",
        "assess",
        "--gate",
        "all",
    ];
    match run_codegate(env, tool_path, &args, timeout) {
        Run::Success(stdout) => match serde_json::from_str::<Value>(&stdout) {
            Ok(assessment) => LanguageEntry {
                assessment: Some(assessment),
                ..LanguageEntry::new(language, Status::Assessed)
            },
            Err(error) => LanguageEntry::failed(
                language,
                format!("{TOOL} assess printed no JSON document"),
                error.to_string(),
            ),
        },
        Run::TimedOut(diagnostic) => LanguageEntry::failed(
            language,
            format!("{TOOL} assess timed out after {} s", timeout.as_secs_f64()),
            diagnostic,
        ),
        Run::Failure(diagnostic) => {
            LanguageEntry::failed(language, format!("{TOOL} assess failed"), diagnostic)
        }
    }
}

/// What one `codegate` call produced; diagnostics are already truncated.
enum Run {
    Success(String),
    Failure(String),
    TimedOut(String),
}

/// The `codegate` children running now, and whether [`shutdown`] has run. One lock guards both,
/// and a child is spawned and registered while it is held, so `shutdown` is a barrier: every
/// child spawned before it is registered and killed, and none is spawned after it.
#[derive(Default)]
struct Children {
    shutting_down: bool,
    groups: HashSet<u32>,
}

static CHILDREN: LazyLock<Mutex<Children>> = LazyLock::new(Default::default);

fn children() -> MutexGuard<'static, Children> {
    CHILDREN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Kill every `codegate` call still running, with its whole process group, and refuse every
/// later one. Installed as a process-exit hook on the first call, so a server that stops
/// (Ctrl-C, SIGTERM, then a normal exit) leaves no assessment behind; callable directly from a
/// shutdown path too. One-way: after it, this process starts no `codegate`.
pub fn shutdown() {
    let mut children = children();
    children.shutting_down = true;
    for group in &children.groups {
        kill_group(*group);
    }
}

extern "C" fn shutdown_at_exit() {
    shutdown();
}

fn install_exit_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        // SAFETY: atexit(3) stores a function pointer; `shutdown_at_exit` is a plain `extern "C"`
        // function that never unwinds (shutdown only locks a mutex and calls kill(2)).
        unsafe {
            libc::atexit(shutdown_at_exit);
        }
    });
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

/// A spawned child's process group, registered in [`CHILDREN`] until dropped.
struct Registered(u32);

impl Registered {
    /// Spawn `command` and register its group under one hold of the lock; refused once
    /// [`shutdown`] has run.
    fn spawn(command: &mut Command) -> Result<(Child, Registered), String> {
        let mut children = children();
        if children.shutting_down {
            return Err("repoview is shutting down".to_owned());
        }
        let child = command.spawn().map_err(|error| error.to_string())?;
        children.groups.insert(child.id());
        let group = child.id();
        Ok((child, Registered(group)))
    }
}

impl Drop for Registered {
    fn drop(&mut self) {
        children().groups.remove(&self.0);
    }
}

/// Run `program args…` as `repoview_sources::run` does (no shell, `current_dir` at the root, the
/// env's `PATH`, its own process group killed at `timeout`), with the group registered for
/// [`shutdown`] while it runs.
fn run_codegate(env: &Env, program: &Path, args: &[&str], timeout: Duration) -> Run {
    install_exit_hook();
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(env.root())
        .env("GIT_OPTIONAL_LOCKS", "0")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = &env.path {
        command.env("PATH", path);
    }
    let (mut child, registered) = match Registered::spawn(&mut command) {
        Ok(spawned) => spawned,
        Err(error) => return failure(format!("{}: {error}", program.display())),
    };
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let timed_out = |group: u32| {
        kill_group(group);
        Run::TimedOut(truncate_diagnostic(&format!(
            "{} timed out after {} ms",
            program.display(),
            timeout.as_millis()
        )))
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let outcome = timed_out(registered.0);
                let _ = child.wait();
                return outcome;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                kill_group(registered.0);
                let _ = child.wait();
                return failure(format!("{}: {error}", program.display()));
            }
        }
    };
    let (Ok(stdout), Ok(stderr)) = (
        stdout.recv_timeout(deadline.saturating_duration_since(Instant::now())),
        stderr.recv_timeout(deadline.saturating_duration_since(Instant::now())),
    ) else {
        return timed_out(registered.0);
    };
    if status.success() {
        Run::Success(stdout)
    } else if stderr.trim().is_empty() {
        failure(format!("{} exited with {status}", program.display()))
    } else {
        failure(stderr)
    }
}

fn failure(text: String) -> Run {
    Run::Failure(truncate_diagnostic(&text))
}

/// Read `pipe` to its end on a thread; the text arrives on the returned channel.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    receiver
}

/// Per server run and project: the assessment run, started exactly once. A `OnceLock` set
/// inside the blocking task, so a request dropped while the run starts cannot release it.
#[derive(Default)]
struct Slot {
    run: OnceLock<Arc<Assessments>>,
}

/// Slots by (run token, project root).
type Slots = HashMap<(String, PathBuf), Arc<Slot>>;

static SLOTS: LazyLock<Mutex<Slots>> = LazyLock::new(Default::default);

fn slot(token: &str, root: &Path) -> Arc<Slot> {
    let mut slots = SLOTS.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        slots
            .entry((token.to_owned(), root.to_path_buf()))
            .or_default(),
    )
}

fn tool_error(status: StatusCode, stderr: &str) -> Response {
    let body = json!({ "tool": TOOL, "exit": null, "stderr": stderr });
    (status, Json(body)).into_response()
}

async fn quality(State(state): State<AppState>, Extension(env): Extension<Env>) -> Response {
    let slot = slot(&state.token, env.root());
    if let Some(run) = slot.run.get() {
        return Json(run.document()).into_response();
    }
    let Some(tool_path) = env.find_tool(TOOL) else {
        return tool_error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("{TOOL} not found on PATH"),
        );
    };
    let started = tokio::task::spawn_blocking(move || {
        Arc::clone(
            slot.run
                .get_or_init(|| Assessments::start(env, tool_path, ASSESS_TIMEOUT)),
        )
    })
    .await;
    match started {
        Ok(run) => Json(run.document()).into_response(),
        Err(error) => tool_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("starting the assessments failed: {error}"),
        ),
    }
}
