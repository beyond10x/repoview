//! Adversary cases for story:quality-codegate: the locator against a `PATH` that names one
//! directory under two names, as a merged-`/usr` system does with `/usr/bin` and `/bin`.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use repoview_sources::{Codegate, Env};

fn stub(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("codegate");
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
    panic!("stub {} stayed busy", path.display());
}

/// `/bin -> usr/bin` with `PATH=/usr/bin:/bin`: one Go `codegate` file, reached through two
/// directory names. The locator documents "each once" and the page lists `skipped` as the
/// binaries passed over, so the one file must be probed once and named once.
#[test]
fn one_go_codegate_reached_through_a_symlinked_directory_is_probed_and_named_once() {
    let root = tempfile::tempdir().unwrap();
    let usr_bin = root.path().join("usr-bin");
    fs::create_dir(&usr_bin).unwrap();
    let log = root.path().join("calls.log");
    let go = stub(
        &usr_bin,
        &format!(
            "[ \"$1\" = probe-busy ] && exit 0\nprintf '%s\\n' \"$*\" >> '{}'\nprintf 'Error: unknown flag: --version\\n' >&2; exit 1",
            log.display()
        ),
    );
    let bin = root.path().join("bin");
    std::os::unix::fs::symlink(&usr_bin, &bin).unwrap();
    let path = std::env::join_paths([&usr_bin, &bin]).unwrap();

    let search = Codegate::locate(&Env::with_path(root.path(), path));

    let calls = fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(
        (search.skipped, calls.lines().count()),
        (vec![go], 1),
        "one file, probed once and named once; calls: {calls:?}"
    );
}
