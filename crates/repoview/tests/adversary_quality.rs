//! Adversary cases for story:quality-page: the background assessment run against a client that
//! goes away mid-request, and against the server shutting down while `codegate assess` runs.

mod common;

use std::fs;
use std::io::{ErrorKind, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use common::{Server, bin_dir, git, repoview};
use repoview::server::TOKEN_HEADER;
use tempfile::TempDir;

const CAPABILITIES: &str = r#"[{"language":"markdown","name":"markdown","capabilities":[]}]"#;
const ASSESSMENT: &str = r#"{"rating":"B-","score_max":100,"scores":{"overall":67}}"#;

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

fn markdown_project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    fs::write(dir.path().join("README.md"), "x\n").unwrap();
    git(dir.path(), &["add", "--", "README.md"]);
    dir
}

fn serve(root: &Path, path: &Path) -> Server {
    Server::start_with(
        repoview()
            .args(["open", "--no-browser", "--port", "0", "--root"])
            .arg(root)
            .env("PATH", path),
    )
}

fn lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn wait_for(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(started.elapsed() < limit, "waited {limit:?} for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A browser reload (or leaving the page) while the first `/api/quality` is still waiting on
/// `codegate capabilities` drops that request. The run it began must still be the only run: a
/// second request must not start a second `capabilities` and a second `assess` per language.
#[test]
fn a_first_request_abandoned_mid_start_does_not_start_a_second_run() {
    let bin = bin_dir(&["git", "sleep"]);
    let log = bin.path().join("calls.log");
    write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
printf '%s\n' "$1 $4" >> '{log}'
if [ "$1" = capabilities ]; then sleep 3; printf '%s' '{CAPABILITIES}'; exit 0; fi
printf '%s' '{ASSESSMENT}'"#,
            log = log.display()
        ),
    );
    let dir = markdown_project();
    let server = serve(dir.path(), bin.path());

    // The first request, abandoned once the server is inside `codegate capabilities`.
    let mut first = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    write!(
        first,
        "GET /api/quality HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{TOKEN_HEADER}: {}\r\n\r\n",
        server.port, server.token
    )
    .unwrap();
    wait_for(
        "the first capabilities call",
        Duration::from_secs(10),
        || {
            lines(&log)
                .iter()
                .any(|line| line.starts_with("capabilities"))
        },
    );
    drop(first);
    std::thread::sleep(Duration::from_millis(300));

    // A second request, as the reloaded page makes it.
    let response = server.get("/api/quality", &[(TOKEN_HEADER, &server.token)]);
    assert_eq!(response.status, 200, "{}", response.text());

    // Long enough for an orphaned first start to finish its capabilities and run its assess.
    std::thread::sleep(Duration::from_secs(4));
    let calls = lines(&log);
    let capabilities = calls
        .iter()
        .filter(|line| line.starts_with("capabilities"))
        .count();
    let assess = calls
        .iter()
        .filter(|line| line.ends_with("markdown"))
        .count();
    assert_eq!(
        (capabilities, assess),
        (1, 1),
        "one run expected, codegate calls: {calls:?}"
    );
}

/// Stopping the server (Ctrl-C, SIGTERM) while an assessment runs must not leave `codegate`
/// running: the child leads its own process group, so nothing else stops it, and with the
/// server gone nothing enforces the 120 s timeout either.
#[test]
fn stopping_the_server_stops_a_running_assessment() {
    let bin = bin_dir(&["git", "sleep"]);
    let pid_file = bin.path().join("assess.pid");
    write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
if [ "$1" = capabilities ]; then printf '%s' '{CAPABILITIES}'; exit 0; fi
echo $$ > '{pid}'
exec sleep 30"#,
            pid = pid_file.display()
        ),
    );
    let dir = markdown_project();
    let mut server = serve(dir.path(), bin.path());
    let response = server.get("/api/quality", &[(TOKEN_HEADER, &server.token)]);
    assert_eq!(response.status, 200, "{}", response.text());
    wait_for("codegate assess to start", Duration::from_secs(10), || {
        fs::read_to_string(&pid_file).is_ok_and(|text| text.trim().parse::<u32>().is_ok())
    });
    let pid: u32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let status = Command::new("kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    wait_for("repoview to exit", Duration::from_secs(10), || {
        server.child.try_wait().unwrap().is_some()
    });
    std::thread::sleep(Duration::from_millis(500));

    let alive =
        fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| !stat.contains(") Z "));
    if alive {
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
    }
    assert!(
        !alive,
        "codegate assess (pid {pid}) is still running after repoview exited"
    );
}
