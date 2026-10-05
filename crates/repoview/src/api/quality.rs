//! `GET /api/quality`: what the beyond10x Codegate on `PATH` can say about the project.
//!
//! Every request locates `codegate` with the same locator the snapshot's `quality` source uses
//! ([`Codegate::locate_with`]); the commands it offers are read from `codegate --help`, never
//! from a list here, once per located binary. The answer names the binary, its version, every
//! `codegate` skipped on the way, and why there is no assessment: no Codegate release has a
//! source assessment yet, so none is started. Every `codegate` child runs with colour off. The
//! project arrives as the `Extension<Env>` that `repoview open` layers over the router.
//!
//! [`BackgroundRun`] is the run machinery kept for the assessment command a later Codegate
//! release adds: started once, read by every later request. Every `codegate` child (the probe's
//! too) leads its own process group and is registered while it runs, so [`shutdown`] (also
//! installed as a process-exit hook) kills whatever is still running when the server stops.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, Once, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum::{Extension, Router};
use repoview_sources::{Codegate, CodegateSearch, Env, Outcome, TIMEOUT, truncate_diagnostic};
use serde::Serialize;
use serde_json::{Value, json};

use crate::server::AppState;

/// The assessing tool, as it is looked up on `PATH` and named on the wire.
pub const TOOL: &str = "codegate";

/// How long one background `codegate` run may take.
pub const ASSESS_TIMEOUT: Duration = Duration::from_secs(120);

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/quality", get(quality))
}

/// The subcommands in the `Commands:` section of `codegate --help`, in order, without clap's own
/// `help`. A command line is indented by exactly two spaces; deeper lines continue a description.
/// ANSI styling (clap colours help under `CLICOLOR_FORCE`) is removed first.
pub fn parse_commands(help: &str) -> Vec<String> {
    strip_ansi(help)
        .lines()
        .skip_while(|line| line.trim_end() != "Commands:")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_owned)
        .collect()
}

/// Why `codegate <version>`, offering `commands`, gives no assessment of the project. Only a
/// codegate offering nothing beyond `evaluate` is said to have no source assessment; any other
/// command may assess, so then the reason says repoview does not read one yet.
pub fn reason(version: &str, commands: &[String]) -> String {
    if commands == ["evaluate"] {
        format!(
            "{TOOL} {version} evaluates supplied dependency facts only; it has no source \
             assessment yet"
        )
    } else if commands.is_empty() {
        format!("{TOOL} {version} offers no commands; it has no source assessment yet")
    } else {
        format!(
            "{TOOL} {version} offers {}; repoview does not read an assessment from it yet",
            commands.join(", ")
        )
    }
}

/// `text` without ANSI escape sequences: CSI (`ESC [ … final`), OSC (`ESC ] … BEL` or
/// `ESC ] … ESC \`) and two-byte escapes.
fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(next) = chars.next() {
        if next != '\u{1b}' {
            plain.push(next);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for byte in chars.by_ref() {
                    if ('@'..='~').contains(&byte) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(byte) = chars.next() {
                    if byte == '\u{7}' {
                        break;
                    }
                    if byte == '\u{1b}' {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    plain
}

/// The binary an [`Offer`] was read from: its path, what its `--version` said, and its file's
/// device, inode, size, modification time and status-change time. A replaced file (a new inode
/// from `mv` or a package manager), a rewrite in place (size, mtime, ctime) and a new version
/// each change it, so `--help` is read again.
#[derive(Debug, PartialEq, Eq)]
struct Identity {
    path: PathBuf,
    version: String,
    file: Option<FileIdentity>,
}

#[derive(Debug, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    len: u64,
    modified: Option<SystemTime>,
    changed: (i64, i64),
}

impl Identity {
    fn of(codegate: &Codegate) -> Identity {
        let file = std::fs::metadata(&codegate.path)
            .ok()
            .map(|meta| FileIdentity {
                dev: meta.dev(),
                ino: meta.ino(),
                len: meta.len(),
                modified: meta.modified().ok(),
                changed: (meta.ctime(), meta.ctime_nsec()),
            });
        Identity {
            path: codegate.path.clone(),
            version: codegate.version.clone(),
            file,
        }
    }
}

/// What one binary offers: the commands its `--help` lists, and the assessment run, if one was
/// started.
struct Offer {
    identity: Identity,
    commands: Vec<String>,
    /// Always `None` until a Codegate release has a source assessment command.
    assessment: Option<Arc<BackgroundRun>>,
}

/// Locate `codegate` with the snapshot's locator, each `--version` registered for [`shutdown`].
fn locate(env: &Env) -> CodegateSearch {
    Codegate::locate_with(env, |candidate| {
        match run_codegate(env, candidate, &["--version"], TIMEOUT) {
            Run::Success(stdout) => Outcome::Success { stdout },
            Run::Failure(diagnostic) | Run::TimedOut(diagnostic) => Outcome::Failure { diagnostic },
        }
    })
}

/// The commands `codegate --help` lists, for the binary identified as `identity` before it ran.
/// The second value says whether the offer may be cached: not when the file changed while
/// `--help` ran, since its output may then be the old binary's. The error is the answer's
/// status and stderr.
fn read_offer(
    env: &Env,
    codegate: &Codegate,
    identity: Identity,
) -> Result<(Offer, bool), (StatusCode, String)> {
    match run_codegate(env, &codegate.path, &["--help"], TIMEOUT) {
        Run::Success(stdout) => {
            let unchanged = Identity::of(codegate) == identity;
            let offer = Offer {
                identity,
                commands: parse_commands(&stdout),
                assessment: None,
            };
            Ok((offer, unchanged))
        }
        Run::Failure(diagnostic) | Run::TimedOut(diagnostic) => {
            Err((StatusCode::BAD_GATEWAY, diagnostic))
        }
    }
}

/// The `/api/quality` document for `codegate`, found after `skipped`, offering `offer`.
fn document(codegate: &Codegate, skipped: &[PathBuf], offer: &Offer) -> Value {
    let skipped: Vec<_> = skipped
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let (assessment, reason) = match &offer.assessment {
        Some(run) => (run.document(), Value::Null),
        None => (
            Value::Null,
            json!(reason(&codegate.version, &offer.commands)),
        ),
    };
    json!({
        "tool": TOOL,
        "tool_path": codegate.path.to_string_lossy(),
        "tool_version": codegate.version,
        "skipped": skipped,
        "commands": offer.commands,
        "assessment": assessment,
        "reason": reason,
    })
}

/// Status of a [`BackgroundRun`] on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Status {
    Assessed,
    Failed,
    Running,
}

#[derive(Debug, Clone, Serialize)]
struct RunDocument {
    status: Status,
    reason: Option<String>,
    stderr: Option<String>,
    assessment: Option<Value>,
}

impl RunDocument {
    fn new(status: Status) -> RunDocument {
        RunDocument {
            status,
            reason: None,
            stderr: None,
            assessment: None,
        }
    }

    fn failed(reason: String, stderr: String) -> RunDocument {
        RunDocument {
            reason: Some(reason),
            stderr: Some(stderr),
            ..RunDocument::new(Status::Failed)
        }
    }
}

/// One background `codegate` run whose stdout is a JSON document: started once, read by every
/// later request.
pub struct BackgroundRun {
    state: Mutex<RunDocument>,
}

impl BackgroundRun {
    /// Start `program args…` on its own thread, bounded by `timeout`, and return at once. The
    /// run is `running` until it finishes, fails or times out.
    pub fn start(
        env: Env,
        program: PathBuf,
        args: Vec<String>,
        timeout: Duration,
    ) -> Arc<BackgroundRun> {
        let run = Arc::new(BackgroundRun {
            state: Mutex::new(RunDocument::new(Status::Running)),
        });
        let running = Arc::clone(&run);
        std::thread::spawn(move || {
            let finished = execute(&env, &program, &args, timeout);
            *running.lock() = finished;
        });
        run
    }

    /// `{ status, reason, stderr, assessment }` as it stands now.
    pub fn document(&self) -> Value {
        serde_json::to_value(&*self.lock()).unwrap_or(Value::Null)
    }

    fn lock(&self) -> MutexGuard<'_, RunDocument> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `program args…`, its JSON passed through unchanged.
fn execute(env: &Env, program: &Path, args: &[String], timeout: Duration) -> RunDocument {
    let label = format!("{TOOL} {}", args.join(" "));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run_codegate(env, program, &args, timeout) {
        Run::Success(stdout) => match serde_json::from_str::<Value>(&stdout) {
            Ok(assessment) => RunDocument {
                assessment: Some(assessment),
                ..RunDocument::new(Status::Assessed)
            },
            Err(error) => RunDocument::failed(
                format!("{label} printed no JSON document"),
                error.to_string(),
            ),
        },
        Run::TimedOut(diagnostic) => RunDocument::failed(
            format!("{label} timed out after {} s", timeout.as_secs_f64()),
            diagnostic,
        ),
        Run::Failure(diagnostic) => RunDocument::failed(format!("{label} failed"), diagnostic),
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
/// env's `PATH`, its own process group killed at `timeout`), with colour off and the group
/// registered for [`shutdown`] while it runs.
fn run_codegate(env: &Env, program: &Path, args: &[&str], timeout: Duration) -> Run {
    install_exit_hook();
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(env.root())
        .env("GIT_OPTIONAL_LOCKS", "0")
        // Plain output whatever the user's shell says: clap colours help under any set
        // CLICOLOR_FORCE, `0` included, unless NO_COLOR is set; so drop it and set NO_COLOR.
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("TERM", "dumb")
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

/// Per server run and project: what the located binary offers. Every request locates `codegate`
/// afresh, as the snapshot does, so the page and the Overview card name the same binary and
/// version, and a removed binary is not found. `--help` runs again only when the located
/// binary's [`Identity`] changed, and an answer read while the file changed is not kept. The
/// lock is taken inside the blocking task and held while the
/// probe runs, so a request dropped while the probe runs cannot release it, and a second request
/// waits for the first probe instead of starting its own.
#[derive(Default)]
struct Slot {
    offer: Mutex<Option<Arc<Offer>>>,
}

impl Slot {
    fn answer(&self, env: &Env) -> Result<Value, (StatusCode, String)> {
        let mut offer = self.offer.lock().unwrap_or_else(PoisonError::into_inner);
        let search = locate(env);
        let Some(codegate) = search.found else {
            *offer = None;
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Codegate::NOT_FOUND.to_owned(),
            ));
        };
        // Taken before `--help` runs, so a binary replaced while it runs is not cached under the
        // new binary's identity.
        let identity = Identity::of(&codegate);
        let current = match &*offer {
            Some(known) if known.identity == identity => Arc::clone(known),
            _ => {
                let (read, unchanged) = read_offer(env, &codegate, identity)?;
                let read = Arc::new(read);
                *offer = unchanged.then(|| Arc::clone(&read));
                read
            }
        };
        Ok(document(&codegate, &search.skipped, &current))
    }
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
    let answered = tokio::task::spawn_blocking(move || slot.answer(&env)).await;
    match answered {
        Ok(Ok(document)) => Json(document).into_response(),
        Ok(Err((status, stderr))) => tool_error(status, &stderr),
        Err(error) => tool_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("probing codegate failed: {error}"),
        ),
    }
}
