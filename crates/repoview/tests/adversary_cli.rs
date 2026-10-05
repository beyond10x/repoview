//! Adversary cases for story:server-skeleton: `--root` validation and browser lookup.

mod common;

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, Instant};

use common::{Server, bin_dir, repoview};

/// `--root <dir>`: a path that exists but is a regular file is not a project directory and must
/// fail like a missing one, not print a snapshot of a "project" named after the file.
#[test]
fn root_pointing_at_a_regular_file_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("README.md");
    std::fs::write(&file, "# not a directory\n").unwrap();
    let output = repoview()
        .args(["snapshot", "--format", "json", "--root"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "exit 0 with --root <file>; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The sources ignore relative `PATH` entries so no binary is run from the project directory.
/// The browser launcher must not either: with an empty `PATH` entry (which `execvp` reads as the
/// working directory) an `xdg-open` committed to the project must not run.
#[cfg(target_os = "linux")]
#[test]
fn browser_launch_does_not_run_an_opener_from_the_project_directory() {
    let project = tempfile::tempdir().unwrap();
    let opener = project.path().join("xdg-open");
    std::fs::write(
        &opener,
        "#!/bin/sh\ncase \"$1\" in http*) echo ran > opener-ran.marker ;; esac\n",
    )
    .unwrap();
    std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(&opener).arg("probe").output() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => break,
        }
    }
    let tools = bin_dir(&["git"]);
    let mut search = std::ffi::OsString::from(":");
    search.push(tools.path());
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .current_dir(project.path())
        .env("PATH", search);
    let server = Server::start_with(&mut command);
    let marker = project.path().join("opener-ran.marker");
    let started = Instant::now();
    while !marker.exists() && started.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(server);
    assert!(
        !marker.exists(),
        "repoview ran ./xdg-open from the project directory"
    );
}
