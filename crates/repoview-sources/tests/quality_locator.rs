//! story:quality-codegate acceptance 1 and 4: locating the beyond10x `codegate` on `PATH`, and the
//! snapshot's `quality` source reporting what the locator found.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use repoview_sources::{Availability, Codegate, CodegateSearch, Env, Section, read_all};
use serde_json::json;
use tempfile::TempDir;

/// What `codegate --version` prints for the beyond10x codegate.
const RUST_STYLE: &str =
    "[ \"$1\" = --version ] && { printf 'codegate 0.3.0\\n'; exit 0; }\nexit 2";

/// What the Go `codegate` does with `--version`: an error and exit 1.
const GO_STYLE: &str = "printf 'Error: unknown flag: --version\\n' >&2; exit 1";

/// Write an executable `/bin/sh` stub and wait until it can be executed (no ETXTBSY from a
/// concurrently forked test thread still holding the write descriptor).
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

/// One `PATH` directory per body, in order, each holding a `codegate` stub.
fn path_of(bodies: &[&str]) -> (Vec<TempDir>, Vec<PathBuf>, OsString) {
    let dirs: Vec<TempDir> = bodies
        .iter()
        .map(|_| tempfile::tempdir().unwrap())
        .collect();
    let stubs = dirs
        .iter()
        .zip(bodies)
        .map(|(dir, body)| stub(dir.path(), body))
        .collect();
    let path = std::env::join_paths(dirs.iter().map(TempDir::path)).unwrap();
    (dirs, stubs, path)
}

fn locate(path: &OsString) -> CodegateSearch {
    let project = tempfile::tempdir().unwrap();
    Codegate::locate(&Env::with_path(project.path(), path))
}

fn quality(path: &OsString) -> Section {
    let project = tempfile::tempdir().unwrap();
    read_all(&Env::with_path(project.path(), path))
        .into_iter()
        .find(|section| section.source_id == "quality")
        .unwrap()
}

#[test]
fn a_rust_style_codegate_first_is_taken_with_its_version() {
    let (_dirs, stubs, path) = path_of(&[RUST_STYLE, GO_STYLE]);
    let search = locate(&path);
    let found = search.found.expect("a codegate");
    assert_eq!(found.path, stubs[0]);
    assert_eq!(found.version, "0.3.0");
    assert!(search.skipped.is_empty(), "{:?}", search.skipped);
}

#[test]
fn a_go_style_codegate_first_is_skipped_and_the_rust_style_one_taken() {
    let (_dirs, stubs, path) = path_of(&[GO_STYLE, RUST_STYLE]);
    let search = locate(&path);
    let found = search.found.expect("a codegate");
    assert_eq!(found.path, stubs[1]);
    assert_eq!(found.version, "0.3.0");
    assert_eq!(search.skipped, vec![stubs[0].clone()]);
}

#[test]
fn only_a_go_style_codegate_is_not_found_and_names_it_skipped() {
    let (_dirs, stubs, path) = path_of(&[GO_STYLE]);
    let search = locate(&path);
    assert!(search.found.is_none(), "{:?}", search.found);
    assert_eq!(search.skipped, stubs);
}

#[test]
fn no_codegate_on_path_is_not_found_with_nothing_skipped() {
    let (_dirs, _stubs, path) = path_of(&[]);
    let search = locate(&path);
    assert!(search.found.is_none());
    assert!(search.skipped.is_empty());
}

#[test]
fn a_codegate_that_exits_0_without_a_semver_is_skipped() {
    let (_dirs, stubs, path) = path_of(&[
        "printf 'codegate\\n'",
        "printf 'other 1.2.3\\n'",
        "printf 'codegate 1.2\\n'",
        "printf 'codegate 1.2.3\\n' >&2",
        RUST_STYLE,
    ]);
    let search = locate(&path);
    assert_eq!(search.found.expect("a codegate").path, stubs[4]);
    assert_eq!(search.skipped, stubs[..4].to_vec());
}

#[test]
fn a_relative_path_entry_is_never_searched() {
    let project = tempfile::tempdir().unwrap();
    let bin = project.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stub(&bin, RUST_STYLE);
    let search = Codegate::locate(&Env::with_path(project.path(), "bin"));
    assert!(search.found.is_none(), "{:?}", search.found);
    assert!(search.skipped.is_empty());
}

#[test]
fn a_directory_listed_twice_is_probed_once() {
    let dir = tempfile::tempdir().unwrap();
    let go = stub(dir.path(), GO_STYLE);
    let path = std::env::join_paths([dir.path(), dir.path()]).unwrap();
    assert_eq!(locate(&path).skipped, vec![go]);
}

#[test]
fn versions_are_codegate_and_a_semver_on_stdout() {
    for (stdout, version) in [
        ("codegate 0.3.0\n", Some("0.3.0")),
        ("codegate 1.20.3-rc.1+build.5", Some("1.20.3-rc.1+build.5")),
        ("codegate 0.3", None),
        ("codegate 01.2.3", None),
        ("codegate v0.3.0", None),
        ("codegate 0.3.0 extra", None),
        ("codegate-go 0.3.0", None),
        ("", None),
    ] {
        assert_eq!(
            Codegate::parse_version(stdout).as_deref(),
            version,
            "{stdout:?}"
        );
    }
}

#[test]
fn the_quality_source_reports_the_located_binary_and_version() {
    let (_dirs, stubs, path) = path_of(&[GO_STYLE, RUST_STYLE]);
    let section = quality(&path);
    assert_eq!(section.availability, Availability::Present);
    assert_eq!(section.tool.as_deref(), Some("codegate"));
    assert_eq!(section.tool_path.as_deref(), stubs[1].to_str());
    assert_eq!(section.tool_version.as_deref(), Some("0.3.0"));
    assert_eq!(
        section.summary,
        json!({ "skipped": [stubs[0].to_str().unwrap()] })
    );
}

#[test]
fn the_quality_source_is_absent_with_only_a_go_style_codegate_and_names_it() {
    let (_dirs, stubs, path) = path_of(&[GO_STYLE]);
    let section = quality(&path);
    assert_eq!(section.availability, Availability::Absent);
    assert_eq!(section.tool_path, None);
    assert_eq!(section.tool_version, None);
    assert_eq!(
        section.summary,
        json!({ "skipped": [stubs[0].to_str().unwrap()] })
    );
    let diagnostic = section
        .diagnostic
        .expect("a diagnostic naming the skipped binary");
    assert!(
        diagnostic.contains(Codegate::NOT_FOUND) && diagnostic.contains(stubs[0].to_str().unwrap()),
        "{diagnostic}"
    );
}

#[test]
fn the_not_found_text_names_the_beyond10x_codegate() {
    assert_eq!(Codegate::NOT_FOUND, "beyond10x codegate not found on PATH");
}
