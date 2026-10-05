//! Round 1: `shutdown()` stops every running `codegate` call. Its own test binary, because
//! `shutdown()` stops every assessment in the process.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use common::{bin_dir, git};
use repoview::api::quality::{ASSESS_TIMEOUT, Assessments, shutdown};
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

fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| !stat.contains(") Z "))
}

#[test]
fn shutdown_kills_a_running_assessment_and_its_children() {
    let bin = bin_dir(&["git", "sleep"]);
    let pids = bin.path().join("pids");
    let codegate = write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
if [ "$1" = capabilities ]; then printf '%s' '[{{"language":"markdown"}}]'; exit 0; fi
sleep 30 &
echo "$$ $!" > '{pids}'
wait"#,
            pids = pids.display()
        ),
    );
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    fs::write(dir.path().join("README.md"), "x\n").unwrap();
    git(dir.path(), &["add", "--", "README.md"]);

    let assessments = Assessments::start(
        Env::with_path(dir.path(), bin.path()),
        codegate,
        ASSESS_TIMEOUT,
    );
    let started = Instant::now();
    let (shell, sleeper) = loop {
        if let Some((a, b)) = fs::read_to_string(&pids).ok().and_then(|text| {
            let mut parts = text.split_whitespace().map(str::parse::<u32>);
            Some((parts.next()?.ok()?, parts.next()?.ok()?))
        }) {
            break (a, b);
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "assess never started"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(alive(shell) && alive(sleeper));

    shutdown();

    let started = Instant::now();
    while alive(shell) || alive(sleeper) {
        if started.elapsed() > Duration::from_secs(5) {
            let _ = Command::new("kill")
                .args(["-KILL", &shell.to_string(), &sleeper.to_string()])
                .status();
            panic!("codegate ({shell}) or its child ({sleeper}) survived shutdown()");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let started = Instant::now();
    while assessments.document()["languages"][0]["status"] == "running" {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(assessments.document()["languages"][0]["status"], "failed");
}
