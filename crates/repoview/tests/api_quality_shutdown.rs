//! Round 1 of story:quality-page, kept by story:quality-codegate acceptance 5: `shutdown()` stops
//! every running `codegate` call. Its own test binary, because `shutdown()` stops every run in the
//! process.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use common::bin_dir;
use repoview::api::quality::{ASSESS_TIMEOUT, BackgroundRun, shutdown};
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
    let bin = bin_dir(&["sleep"]);
    let pids = bin.path().join("pids");
    let codegate = write_stub(
        bin.path(),
        "codegate",
        &format!(
            r#"[ "$1" = probe-busy ] && exit 0
sleep 30 &
echo "$$ $!" > '{pids}'
wait"#,
            pids = pids.display()
        ),
    );
    let dir = tempfile::tempdir().unwrap();

    let run = BackgroundRun::start(
        Env::with_path(dir.path(), bin.path()),
        codegate,
        vec!["assess".to_owned()],
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
    while run.document()["status"] == "running" {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(run.document()["status"], "failed");
}
