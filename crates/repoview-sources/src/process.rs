//! Tool lookup and subprocesses: no shell, `current_dir` at the project root, a timeout each.

use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use wait_timeout::ChildExt;

use crate::Env;

/// How long one tool invocation may run.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The most of a tool's stderr a diagnostic carries, in bytes.
pub const DIAGNOSTIC_LIMIT: usize = 4096;

/// What one tool invocation produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Exit status 0.
    Success { stdout: String },
    /// Non-zero exit, a signal, a timeout or a spawn error; `diagnostic` is already truncated.
    Failure { diagnostic: String },
}

/// The first executable file called `name` in the absolute directories of `path`. Relative
/// entries, including the empty one `execvp` reads as the working directory, are skipped.
pub fn find_tool(path: Option<&OsStr>, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .find(|candidate| {
            candidate
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// Everything one tool invocation produced, whatever its exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The exit code; `None` when a signal ended the tool, it timed out or it did not start.
    pub exit: Option<i32>,
    /// The tool's stdout, byte for byte; empty when it timed out or did not start.
    pub stdout: Vec<u8>,
    /// The tool's stderr in full; when it timed out or did not start, repoview's diagnostic
    /// instead, already cut to [`DIAGNOSTIC_LIMIT`].
    pub stderr: String,
    /// The deadline passed and the tool's process group was killed.
    pub timed_out: bool,
    /// How the tool ended; `None` exactly when it timed out or did not start.
    pub status: Option<ExitStatus>,
}

impl Output {
    /// A run that never reached an exit: a timeout or a spawn error.
    fn unfinished(diagnostic: &str, timed_out: bool) -> Self {
        Output {
            exit: None,
            stdout: Vec::new(),
            stderr: truncate_diagnostic(diagnostic),
            timed_out,
            status: None,
        }
    }
}

/// Run `program args…` in `env.root` with `env`'s `PATH`: [`run_output`], read as success or
/// failure.
///
/// Exit status 0 is [`Outcome::Success`] with stdout. Anything else is [`Outcome::Failure`]: the
/// tool's stderr, or `<program> exited with <status>` when stderr is blank, or the timeout or
/// spawn diagnostic.
pub fn run(env: &Env, program: &Path, args: &[&str], timeout: Duration) -> Outcome {
    let output = run_output(env, program, args, timeout);
    match output.status {
        None => Outcome::Failure {
            diagnostic: output.stderr,
        },
        Some(status) if status.success() => Outcome::Success {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        },
        Some(status) if output.stderr.trim().is_empty() => {
            failure(&format!("{} exited with {status}", program.display()))
        }
        Some(_) => failure(&output.stderr),
    }
}

/// Run `program args…` in `env.root` with `env`'s `PATH`, no shell, stdin closed, and keep the
/// exit code and both streams whatever the exit.
///
/// `timeout` bounds the whole run, output included: the child leads its own process group, and
/// at the one deadline the group is killed, so a background grandchild holding a pipe open cannot
/// keep the call waiting. Every child runs with `GIT_OPTIONAL_LOCKS=0` and, through
/// `GIT_CONFIG_COUNT`, `core.fsmonitor=false` and `core.untrackedCache=false`, so no `git`
/// invocation rewrites the index, starts an fsmonitor daemon or writes a cache into `.git`.
pub fn run_output(env: &Env, program: &Path, args: &[&str], timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&env.root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "core.fsmonitor")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "core.untrackedCache")
        .env("GIT_CONFIG_VALUE_1", "false")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = &env.path {
        command.env("PATH", path);
    }
    let os_error = |error: &dyn std::fmt::Display| {
        Output::unfinished(&format!("{}: {error}", program.display()), false)
    };
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return os_error(&error),
    };
    let group = child.id();
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let timed_out = || {
        kill_group(group);
        Output::unfinished(
            &format!(
                "{} timed out after {} ms",
                program.display(),
                timeout.as_millis()
            ),
            true,
        )
    };
    let status = match child.wait_timeout(remaining(deadline)) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let outcome = timed_out();
            let _ = child.wait();
            return outcome;
        }
        Err(error) => {
            kill_group(group);
            let _ = child.wait();
            return os_error(&error);
        }
    };
    let (Ok(stdout), Ok(stderr)) = (
        stdout.recv_timeout(remaining(deadline)),
        stderr.recv_timeout(remaining(deadline)),
    ) else {
        return timed_out();
    };
    Output {
        exit: status.code(),
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        timed_out: false,
        status: Some(status),
    }
}

fn remaining(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
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

/// Read `pipe` to its end on a thread; the bytes arrive on the returned channel.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(bytes);
    });
    receiver
}

fn failure(text: &str) -> Outcome {
    Outcome::Failure {
        diagnostic: truncate_diagnostic(text),
    }
}

/// `text` cut to at most [`DIAGNOSTIC_LIMIT`] bytes on a character boundary.
pub fn truncate_diagnostic(text: &str) -> String {
    if text.len() <= DIAGNOSTIC_LIMIT {
        return text.to_owned();
    }
    let end = text.floor_char_boundary(DIAGNOSTIC_LIMIT);
    text[..end].to_owned()
}
