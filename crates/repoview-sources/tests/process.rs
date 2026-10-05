//! `run_output`, the one subprocess runner every source and API module uses, and `run` rebuilt on
//! it with its messages unchanged.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use repoview_sources::{DIAGNOSTIC_LIMIT, Env, Outcome, Output, run, run_output};
use tempfile::TempDir;

/// The spawn error of a script a concurrently forked test thread still holds open for writing.
const BUSY: &str = "Text file busy";

/// A project directory holding an executable `/bin/sh` script `tool` with `body`.
fn project_with_tool(body: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tool");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    (dir, path)
}

/// `run_output` in `env`, retried while the script is busy, with the time the last try took.
fn run_output_in(env: &Env, tool: &Path, args: &[&str], timeout: Duration) -> (Output, Duration) {
    for _ in 0..200 {
        let started = Instant::now();
        let output = run_output(env, tool, args, timeout);
        if output.status.is_none() && !output.timed_out && output.stderr.contains(BUSY) {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        return (output, started.elapsed());
    }
    panic!("the script stayed busy");
}

/// `run_output` in the project `dir`.
fn run_tool(dir: &Path, tool: &Path, args: &[&str], timeout: Duration) -> (Output, Duration) {
    run_output_in(&Env::new(dir), tool, args, timeout)
}

/// `run` in the project `dir`, retried while the script is busy.
fn run_in(dir: &Path, tool: &Path, timeout: Duration) -> Outcome {
    for _ in 0..200 {
        match run(&Env::new(dir), tool, &[], timeout) {
            Outcome::Failure { diagnostic } if diagnostic.contains(BUSY) => {
                thread::sleep(Duration::from_millis(10))
            }
            outcome => return outcome,
        }
    }
    panic!("the script stayed busy");
}

/// Whether a process `pid` still exists and is not a zombie.
fn alive(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| !rest.starts_with('Z'))
    })
}

#[test]
fn the_exit_code_and_both_streams_are_reported_whatever_the_exit() {
    let (dir, tool) = project_with_tool("printf out\nprintf err >&2\nexit 7");
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(output.exit, Some(7), "{output:?}");
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, "err");
    assert!(!output.timed_out);
    assert_eq!(output.status.and_then(|status| status.code()), Some(7));
}

#[test]
fn exit_zero_keeps_stdout_and_stderr() {
    let (dir, tool) = project_with_tool("printf '{\"a\":1}'\nprintf note >&2");
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(
        output,
        Output {
            exit: Some(0),
            stdout: b"{\"a\":1}".to_vec(),
            stderr: "note".to_owned(),
            timed_out: false,
            status: Some(std::process::ExitStatus::from_raw(0)),
        }
    );
}

/// stdout is the tool's bytes, unchanged, so a caller that passes it through answers with exactly
/// what the tool printed; invalid UTF-8 is not replaced.
#[test]
fn stdout_is_kept_byte_for_byte() {
    let (dir, tool) = project_with_tool("printf '{\"a\":\"\\377\"}'");
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(output.stdout, b"{\"a\":\"\xff\"}", "{output:?}");
}

#[test]
fn the_arguments_reach_the_tool_unchanged_and_without_a_shell() {
    let (dir, tool) = project_with_tool("for a in \"$@\"; do printf '[%s]' \"$a\"; done");
    let (output, _) = run_tool(
        dir.path(),
        &tool,
        &["a b", "$HOME", "--", ";x"],
        Duration::from_secs(5),
    );
    assert_eq!(output.stdout, b"[a b][$HOME][--][;x]", "{output:?}");
}

#[test]
fn a_signal_is_no_exit_code_and_not_a_timeout() {
    let (dir, tool) = project_with_tool("printf partial\nkill -9 $$");
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(output.exit, None, "{output:?}");
    assert!(!output.timed_out);
    assert_eq!(output.stdout, b"partial");
    assert_eq!(output.status.and_then(|status| status.signal()), Some(9));
}

#[test]
fn a_hung_tool_is_killed_at_the_deadline_even_with_a_grandchild_holding_the_pipe() {
    let (dir, tool) = project_with_tool("sleep 30 &\necho $! > grandchild.pid\nsleep 30");
    let (output, elapsed) = run_tool(dir.path(), &tool, &[], Duration::from_millis(300));
    assert!(output.timed_out, "{output:?}");
    assert_eq!(output.exit, None);
    assert_eq!(output.status, None);
    assert!(
        output.stderr.contains("timed out after 300 ms"),
        "{}",
        output.stderr
    );
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    let pid: i32 = fs::read_to_string(dir.path().join("grandchild.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let started = Instant::now();
    while alive(pid) && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !alive(pid),
        "the grandchild {pid} outlived the process-group kill"
    );
}

#[test]
fn a_grandchild_holding_the_pipe_after_exit_is_bounded_too() {
    let (dir, tool) = project_with_tool("sleep 30 &\nexit 0");
    let (output, elapsed) = run_tool(dir.path(), &tool, &[], Duration::from_millis(300));
    assert!(output.timed_out, "{output:?}");
    assert_eq!(output.exit, None);
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
}

/// The wait for the exit and the reads of both pipes share one deadline: a tool that exits just
/// before it, leaving a grandchild on the pipe, does not get a second full timeout for its output.
#[test]
fn waiting_and_reading_share_one_deadline() {
    let (dir, tool) = project_with_tool("sleep 30 &\nsleep 0.8\nexit 0");
    let (output, elapsed) = run_tool(dir.path(), &tool, &[], Duration::from_millis(1000));
    assert!(output.timed_out, "{output:?}");
    assert!(elapsed < Duration::from_millis(1700), "{elapsed:?}");
}

#[test]
fn a_tool_that_cannot_start_is_neither_an_exit_nor_a_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-tool");
    let (output, _) = run_tool(dir.path(), &missing, &[], Duration::from_secs(5));
    assert_eq!(output.exit, None, "{output:?}");
    assert_eq!(output.status, None);
    assert!(!output.timed_out);
    assert_eq!(output.stdout, b"");
    assert!(
        output
            .stderr
            .starts_with(&format!("{}: ", missing.display())),
        "{}",
        output.stderr
    );
}

#[test]
fn the_child_runs_in_the_project_root_with_the_env_path_and_read_only_git() {
    // `pwd` is a shell builtin; `env` is named absolutely because `PATH` is an empty directory.
    let (dir, tool) = project_with_tool("pwd\n/usr/bin/env");
    let bin = tempfile::tempdir().unwrap();
    let env = Env::with_path(dir.path(), bin.path());
    let (output, _) = run_output_in(&env, &tool, &[], Duration::from_secs(5));
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let mut lines = stdout.lines();
    assert_eq!(
        fs::canonicalize(lines.next().unwrap()).unwrap(),
        fs::canonicalize(dir.path()).unwrap()
    );
    let lines: Vec<&str> = lines.collect();
    let path_line = format!("PATH={}", bin.path().display());
    for expected in [
        path_line.as_str(),
        "GIT_OPTIONAL_LOCKS=0",
        "GIT_CONFIG_COUNT=2",
        "GIT_CONFIG_KEY_0=core.fsmonitor",
        "GIT_CONFIG_VALUE_0=false",
        "GIT_CONFIG_KEY_1=core.untrackedCache",
        "GIT_CONFIG_VALUE_1=false",
    ] {
        assert!(lines.contains(&expected), "{expected} missing:\n{output:?}");
    }
}

#[test]
fn stdin_is_closed() {
    let (dir, tool) = project_with_tool("cat; printf done");
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(output.stdout, b"done", "{output:?}");
    assert!(!output.timed_out);
}

/// `run_output` keeps a tool's streams whole; only `run`'s diagnostic is cut to the limit.
#[test]
fn the_streams_are_not_truncated_but_the_run_diagnostic_is() {
    let body = format!(
        "head -c {n} /dev/zero | tr '\\0' o\nhead -c {n} /dev/zero | tr '\\0' e >&2\nexit 1",
        n = DIAGNOSTIC_LIMIT * 3
    );
    let (dir, tool) = project_with_tool(&body);
    let (output, _) = run_tool(dir.path(), &tool, &[], Duration::from_secs(5));
    assert_eq!(output.stdout.len(), DIAGNOSTIC_LIMIT * 3);
    assert_eq!(output.stderr.len(), DIAGNOSTIC_LIMIT * 3);
    match run_in(dir.path(), &tool, Duration::from_secs(5)) {
        Outcome::Failure { diagnostic } => assert_eq!(diagnostic, "e".repeat(DIAGNOSTIC_LIMIT)),
        other => panic!("{other:?}"),
    }
}

// `run` on top of `run_output`: the messages it gave before.

#[test]
fn run_succeeds_with_stdout_on_exit_zero() {
    let (dir, tool) = project_with_tool("printf out\nprintf err >&2");
    assert_eq!(
        run_in(dir.path(), &tool, Duration::from_secs(5)),
        Outcome::Success {
            stdout: "out".to_owned()
        }
    );
}

#[test]
fn run_reports_stderr_on_a_non_zero_exit() {
    let (dir, tool) = project_with_tool("printf out\nprintf 'it broke' >&2\nexit 3");
    assert_eq!(
        run_in(dir.path(), &tool, Duration::from_secs(5)),
        Outcome::Failure {
            diagnostic: "it broke".to_owned()
        }
    );
}

#[test]
fn run_reports_the_exit_status_when_stderr_is_blank() {
    let (dir, tool) = project_with_tool("printf out\nprintf '  \\n' >&2\nexit 3");
    assert_eq!(
        run_in(dir.path(), &tool, Duration::from_secs(5)),
        Outcome::Failure {
            diagnostic: format!("{} exited with exit status: 3", tool.display())
        }
    );
}

#[test]
fn run_reports_the_signal_when_stderr_is_blank() {
    let (dir, tool) = project_with_tool("kill -9 $$");
    assert_eq!(
        run_in(dir.path(), &tool, Duration::from_secs(5)),
        Outcome::Failure {
            diagnostic: format!("{} exited with signal: 9 (SIGKILL)", tool.display())
        }
    );
}

#[test]
fn run_reports_a_timeout() {
    let (dir, tool) = project_with_tool("sleep 30");
    assert_eq!(
        run_in(dir.path(), &tool, Duration::from_millis(200)),
        Outcome::Failure {
            diagnostic: format!("{} timed out after 200 ms", tool.display())
        }
    );
}

#[test]
fn run_reports_a_tool_that_cannot_start() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-tool");
    match run_in(dir.path(), &missing, Duration::from_secs(5)) {
        Outcome::Failure { diagnostic } => assert!(
            diagnostic.starts_with(&format!("{}: ", missing.display())),
            "{diagnostic}"
        ),
        other => panic!("{other:?}"),
    }
}
