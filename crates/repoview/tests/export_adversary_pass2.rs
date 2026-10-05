//! Adversary cases for story:static-export, pass 2 (after correction round 1).
//!
//! `Scrub` promises that "only whole path components are rewritten" (tests/export.rs
//! `a_home_path_inside_a_longer_name_is_left_alone`). The root and home rewrite treats `.` as the
//! end of a component, so a sibling such as `proj.next` loses its name; the tool rewrite has no
//! component boundary at all on its left, so a tool path inside a longer path is spliced.
//!
//! `--out` "is refused if a symlink"; spelt with a trailing slash, as the story itself spells it
//! (`repoview export --out site/`), the symlink is followed instead.

mod common;

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use repoview::export::{self, Error, Options, Site};
use repoview_sources::Env;
use serde_json::Value;

use common::{git, real_tool};

const TOKEN: &str = "b2b20000b2b20000b2b20000b2b20000b2b20000b2b20000b2b20000b2b20000";

const INDEX: &str = "<!doctype html><html><head>\
<meta name=\"repoview-mode\" content=\"server\" /><title>repoview</title></head>\
<body><div id=\"app\"></div></body></html>\n";

fn site() -> Site {
    Site::new(INDEX.as_bytes().to_vec(), Vec::new())
}

fn options(out: &Path, home: Option<PathBuf>) -> Options {
    Options {
        out: out.to_path_buf(),
        force: false,
        token: TOKEN.to_owned(),
        home,
    }
}

/// `git worktree add ../proj.next` is a common way to keep a second checkout beside the project.
/// The vcs page lists it; exported, its path must still name `proj.next`, not `.` plus `.next`.
#[test]
fn a_sibling_worktree_whose_name_extends_the_roots_keeps_its_own_name() {
    let home = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(home.path()).unwrap();
    let project = home.join("proj");
    let bin = home.join("bin");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&bin).unwrap();
    symlink(real_tool("git"), bin.join("git")).unwrap();
    fs::write(project.join("README.md"), "# x\n").unwrap();
    git(&project, &["init", "--quiet"]);
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "--quiet", "-m", "first"]);
    git(
        &project,
        &["worktree", "add", "--quiet", "-b", "next", "../proj.next"],
    );

    let out = home.join("site");
    let env = Env::with_path(&project, bin.into_os_string());
    export::export(&env, &site(), &options(&out, Some(home.clone()))).expect("export succeeds");

    let vcs: Value =
        serde_json::from_str(&fs::read_to_string(out.join("data/vcs.json")).unwrap()).unwrap();
    let paths: Vec<&str> = vcs["worktrees"]
        .as_array()
        .expect("vcs.json lists worktrees")
        .iter()
        .filter_map(|worktree| worktree["path"].as_str())
        .collect();
    assert!(
        paths.iter().any(|path| path.ends_with("proj.next")),
        "the sibling worktree {}/proj.next is exported as {paths:?}",
        "~"
    );
}

#[test]
fn a_root_or_home_spelling_followed_by_a_dot_is_not_a_whole_component() {
    let scrub = export::Scrub::new(Path::new("/srv/p"), Some(Path::new("/srv/u")), TOKEN);
    // `/srv/p.git` is not under `/srv/p`, as `/srv/ux` is not under `/srv/u`.
    assert_eq!(scrub.text("remote /srv/p.git"), "remote /srv/p.git");
    assert_eq!(scrub.text("/srv/u.old/a"), "/srv/u.old/a");
}

/// With `/bin` before `/usr/bin` on `PATH` (`/bin` -> `usr/bin` on merged-/usr systems, a case
/// the quality locator already handles), `git`'s `tool_path` is `/bin/git`; any `/usr/bin/git`
/// in an exported text is then spliced into `/usrgit`.
#[test]
fn a_tool_path_inside_a_longer_path_is_not_spliced() {
    let scrub = export::Scrub::new(Path::new("/srv/p"), None, TOKEN).tools(["/bin/git"]);
    let text = scrub.text("hooks run /usr/bin/git and /bin/git");
    assert!(
        !text.contains("/usrgit"),
        "a different path was spliced: {text:?}"
    );
    assert!(
        !text.contains("/bin/git"),
        "the tool path survived: {text:?}"
    );
}

#[test]
fn an_out_directory_symlink_spelt_with_a_trailing_slash_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let scratch = fs::canonicalize(scratch.path()).unwrap();
    let real = scratch.join("real");
    fs::create_dir(&real).unwrap();
    let link = scratch.join("site");
    symlink(&real, &link).unwrap();
    let project = scratch.join("proj");
    fs::create_dir_all(&project).unwrap();
    let empty = scratch.join("empty-bin");
    fs::create_dir_all(&empty).unwrap();
    fs::set_permissions(&empty, fs::Permissions::from_mode(0o755)).unwrap();
    let env = Env::with_path(&project, empty.into_os_string());

    let spelt = PathBuf::from(format!("{}/", link.display()));
    let result = export::export(&env, &site(), &options(&spelt, None));
    assert!(
        matches!(result, Err(Error::Refused(_))),
        "--out {} (a symlink to {}) was not refused: {result:?}; {} now holds {:?}",
        spelt.display(),
        real.display(),
        real.display(),
        fs::read_dir(&real)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>()
    );
}
