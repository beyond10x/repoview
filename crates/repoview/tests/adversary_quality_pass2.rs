//! Adversary cases, pass 2, for story:quality-page, rewritten by story:quality-codegate:
//! `shutdown()` (and the exit hook that calls it) against runs that are starting at the moment the
//! server stops. Its own test binary, because `shutdown()` stops every run in the process.

mod common;

use std::fs;
use std::io::{ErrorKind, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{Server, bin_dir, repoview};
use repoview::api::quality::{ASSESS_TIMEOUT, BackgroundRun, shutdown};
use repoview::server::TOKEN_HEADER;
use repoview_sources::Env;

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&path).arg("probe-busy").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => return path,
        }
    }
    panic!("stub {name} stayed busy");
}

/// A `codegate` whose `--version` sleeps `version_sleep` seconds and answers `codegate 0.3.0`,
/// and whose every other call appends its pid to `pids` and then runs for 30 s.
fn codegate(bin: &Path, version_sleep: &str, pids: &Path, log: &Path) -> PathBuf {
    write_stub(
        bin,
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
printf '%s\n' "$1" >> '{log}'
if [ "$1" = --version ]; then sleep {version_sleep}; printf 'codegate 0.3.0\n'; exit 0; fi
echo $$ >> '{pids}'
exec sleep 30"#,
            log = log.display(),
            pids = pids.display(),
        ),
    )
}

fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| !stat.contains(") Z "))
}

/// The pids in `pids` still running; each survivor is SIGKILLed so the test leaves nothing behind.
fn survivors(pids: &Path) -> Vec<u32> {
    let found: Vec<u32> = fs::read_to_string(pids)
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|pid| pid.parse().ok())
        .filter(|&pid| alive(pid))
        .collect();
    for pid in &found {
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
    }
    found
}

/// `BackgroundRun::start` documents that it returns once the run is started, and `shutdown()` that
/// it kills every `codegate` call still running. A shutdown right after five starts returned must
/// therefore stop every run they began, including any whose thread had not spawned yet.
#[test]
fn shutdown_right_after_start_stops_every_run_it_began() {
    let bin = bin_dir(&["sleep"]);
    let pids = bin.path().join("pids");
    let log = bin.path().join("calls.log");
    let tool = codegate(bin.path(), "0", &pids, &log);
    let dir = tempfile::tempdir().unwrap();

    let runs: Vec<Arc<BackgroundRun>> = (0..5)
        .map(|_| {
            BackgroundRun::start(
                Env::with_path(dir.path(), bin.path()),
                tool.clone(),
                vec!["assess".to_owned()],
                ASSESS_TIMEOUT,
            )
        })
        .collect();
    shutdown();

    std::thread::sleep(Duration::from_millis(1500));
    let left = survivors(&pids);
    let documents: Vec<_> = runs.iter().map(|run| run.document()).collect();
    assert!(
        left.is_empty(),
        "codegate still running after shutdown(): pids {left:?}; runs {documents:?}"
    );
}

/// The same race through the served binary: SIGTERM while the first request is inside
/// `codegate --version`. The probe goes on to `--help` during the grace period; when repoview has
/// exited, no `codegate` may be left running. Several attempts, because the window is the time
/// the next call takes to spawn and register.
#[test]
fn sigterm_while_the_run_starts_leaves_no_codegate_running() {
    let mut orphaned = Vec::new();
    for attempt in 0..6 {
        let bin = bin_dir(&["sleep"]);
        let pids = bin.path().join("pids");
        let log = bin.path().join("calls.log");
        codegate(bin.path(), "1", &pids, &log);
        let dir = tempfile::tempdir().unwrap();
        let mut server = Server::start_with(
            repoview()
                .args(["open", "--no-browser", "--port", "0", "--root"])
                .arg(dir.path())
                .env("PATH", bin.path()),
        );
        let mut request = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
        write!(
            request,
            "GET /api/quality HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{TOKEN_HEADER}: {}\r\n\r\n",
            server.port, server.token
        )
        .unwrap();
        let started = Instant::now();
        while !fs::read_to_string(&log).is_ok_and(|text| text.contains("--version")) {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "attempt {attempt}: codegate --version never ran"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let status = Command::new("kill")
            .args(["-TERM", &server.child.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());
        let started = Instant::now();
        while server.child.try_wait().unwrap().is_none() {
            assert!(
                started.elapsed() < Duration::from_secs(15),
                "attempt {attempt}: repoview did not exit"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(request);
        std::thread::sleep(Duration::from_millis(500));
        let left = survivors(&pids);
        if !left.is_empty() {
            orphaned.push((attempt, left));
        }
    }
    assert!(
        orphaned.is_empty(),
        "codegate left running after repoview exited (attempt, pids): {orphaned:?}"
    );
}
