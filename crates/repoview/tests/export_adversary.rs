//! Adversary cases for story:static-export.
//!
//! Acceptance 5: `--force` "only removes what a previous export wrote". The manifest's file names
//! are checked lexically (normal components only), but a directory or file inside `--out` that is
//! a symlink is followed, so a removal or a write lands outside `--out`.
//!
//! Acceptance 3: "no `tool_path` values appear in any exported file". A tool's own path also
//! reaches the answers inside failure diagnostics (`<path> exited with …`, `<path> timed out …`),
//! which only the home and root rewrite touches.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use repoview::export::{self, Options, Site};
use repoview_sources::Env;

const TOKEN: &str = "adad0000adad0000adad0000adad0000adad0000adad0000adad0000adad0000";

const INDEX: &str = "<!doctype html><html><head>\
<meta name=\"repoview-mode\" content=\"server\" /><title>repoview</title></head>\
<body><div id=\"app\"></div></body></html>\n";

fn site() -> Site {
    Site::new(INDEX.as_bytes().to_vec(), Vec::new())
}

/// A project with nothing a source detects, read with an empty `PATH`.
fn bare_env(scratch: &Path) -> Env {
    let project = scratch.join("proj");
    fs::create_dir_all(&project).unwrap();
    let empty = scratch.join("empty-bin");
    fs::create_dir_all(&empty).unwrap();
    Env::with_path(project, empty.into_os_string())
}

fn options(out: &Path, force: bool) -> Options {
    Options {
        out: out.to_path_buf(),
        force,
        token: TOKEN.to_owned(),
        home: None,
    }
}

/// Every regular file under `dir`, relative, `/`-separated (symlinks are not followed).
fn files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                out.push(path.strip_prefix(dir).unwrap().to_str().unwrap().to_owned());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn force_never_removes_a_file_outside_out_through_a_symlinked_directory() {
    let scratch = tempfile::tempdir().unwrap();
    let scratch = fs::canonicalize(scratch.path()).unwrap();
    let outside = scratch.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("victim.txt"), "precious").unwrap();

    // A directory that looks like a previous export: a manifest whose every name is a relative
    // path of normal components, and `link` a symlink inside --out that points out of it.
    let out = scratch.join("site");
    fs::create_dir_all(out.join("data")).unwrap();
    symlink("../outside", out.join("link")).unwrap();
    fs::write(
        out.join("data/export.json"),
        r#"{"repoview_version":"0.1.0","files":["link/victim.txt"]}"#,
    )
    .unwrap();

    let result = export::export(&bare_env(&scratch), &site(), &options(&out, true));
    assert_eq!(
        fs::read_to_string(outside.join("victim.txt"))
            .ok()
            .as_deref(),
        Some("precious"),
        "--force removed {} although it is outside --out {} (export returned {result:?})",
        outside.join("victim.txt").display(),
        out.display()
    );
}

#[test]
fn force_never_writes_through_a_symlink_to_a_file_outside_out() {
    let scratch = tempfile::tempdir().unwrap();
    let scratch = fs::canonicalize(scratch.path()).unwrap();
    let outside = scratch.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("victim.txt"), "precious").unwrap();

    // The previous manifest does not list index.html, so it is not removed; it is a symlink to a
    // file outside --out, and the export then writes index.html.
    let out = scratch.join("site");
    fs::create_dir_all(out.join("data")).unwrap();
    symlink("../outside/victim.txt", out.join("index.html")).unwrap();
    fs::write(
        out.join("data/export.json"),
        r#"{"repoview_version":"0.1.0","files":["data/export.json"]}"#,
    )
    .unwrap();

    let result = export::export(&bare_env(&scratch), &site(), &options(&out, true));
    assert_eq!(
        fs::read_to_string(outside.join("victim.txt"))
            .ok()
            .as_deref(),
        Some("precious"),
        "the export overwrote {} outside --out {} (export returned {result:?})",
        outside.join("victim.txt").display(),
        out.display()
    );
}

/// `aep` that fails `--version` with an empty stderr, as a tool does when it is killed or exits
/// without a message: the snapshot's diagnostic is then `<tool path> exited with …`.
const SILENT_AEP: &str = "#!/bin/sh\nexit 1\n";

#[test]
fn no_exported_file_holds_a_tool_path_outside_home_in_a_diagnostic() {
    let scratch = tempfile::tempdir().unwrap();
    let scratch = fs::canonicalize(scratch.path()).unwrap();
    // The tool lives outside both the project and the home directory, as /usr/local/bin or
    // /opt/homebrew/bin do.
    let bin = scratch.join("opt/tools/bin");
    fs::create_dir_all(&bin).unwrap();
    let aep = bin.join("aep");
    fs::write(&aep, SILENT_AEP).unwrap();
    fs::set_permissions(&aep, fs::Permissions::from_mode(0o755)).unwrap();
    let project = scratch.join("proj");
    fs::create_dir_all(project.join(".engineering")).unwrap();
    fs::write(project.join(".engineering/project.yaml"), "name: fixture\n").unwrap();
    let home = scratch.join("home/someone");
    fs::create_dir_all(&home).unwrap();

    let out = scratch.join("site");
    let env = Env::with_path(&project, bin.clone().into_os_string());
    let options = Options {
        home: Some(home),
        ..options(&out, false)
    };
    export::export(&env, &site(), &options).expect("export succeeds");

    let tool_path: PathBuf = aep;
    let tool_path = tool_path.to_str().unwrap();
    let mut holding = Vec::new();
    for file in files(&out) {
        let text = String::from_utf8_lossy(&fs::read(out.join(&file)).unwrap()).into_owned();
        if text.contains(tool_path) {
            holding.push(file);
        }
    }
    assert!(
        holding.is_empty(),
        "the tool_path value {tool_path} appears in {holding:?}; snapshot: {}",
        fs::read_to_string(out.join("data/snapshot.json")).unwrap_or_default()
    );
}
