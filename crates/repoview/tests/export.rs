//! story:static-export: `repoview export --out <dir>` writes the SPA in static mode and one JSON
//! file per API answer the pages read, from the real router, with no token and no home paths.
//!
//! The fixture is a project inside a stand-in home directory, with stub `aep` and `ess` and the
//! real `git` linked into a `PATH` directory that also lives in that home, so tool paths, tool
//! output and the snapshot's project root all carry the home path the export must not leak.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use repoview::assets::MemoryAssets;
use repoview::export::{self, Error, Options, Site};
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

use common::{git, real_tool, repoview};

const TOKEN: &str = "7e57a0ce7e57a0ce7e57a0ce7e57a0ce7e57a0ce7e57a0ce7e57a0ce7e57a0ce";

const INDEX: &str = "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"UTF-8\" />\n    \
<meta name=\"repoview-mode\" content=\"server\" />\n    <title>repoview</title>\n    \
<script type=\"module\" crossorigin src=\"./assets/index-abc12345.js\"></script>\n  </head>\n  \
<body><div id=\"app\"></div></body>\n</html>\n";

const AEP: &str = r#"#!/bin/sh
[ "$1" = --version ] && { echo "aep 9.9.9"; exit 0; }
verb="$3"
id=""
for arg in "$@"; do id="$arg"; done
case "$verb" in
  list) printf '[{"id":"story:one","kind":"story","status":"active","title":"One","path":"story/one.md","relations":[],"refs":[],"blocked_by":[]},{"id":"epic:two","kind":"epic","status":"draft","title":"Two","path":"epic/two.md","relations":[],"refs":[],"blocked_by":[]}]' ;;
  board) printf '[{"status":"active","items":[]}]' ;;
  graph) printf '{"nodes":[],"edges":[]}' ;;
  validate) printf '{"problems":["story:one has no acceptance"]}'; echo "invalid store at $PWD" >&2; exit 1 ;;
  show) printf '{"id":"%s","kind":"story","status":"active","title":"t","path":"%s/.engineering/planning/x.md","relations":[],"refs":[],"blocked_by":[]}' "$id" "$PWD" ;;
  history) printf '[{"revision":1,"at":"2026-10-05T00:00:00Z"}]' ;;
  explain)
    if [ "$id" = epic:two ]; then echo "explain failed in $PWD" >&2; exit 3; fi
    printf '{"artifact":"%s"}' "$id" ;;
  *) printf '{}' ;;
esac
"#;

const ESS: &str = r#"#!/bin/sh
[ "$1" = --version ] && { echo "ess 9.9.9"; exit 0; }
case "$2" in
  validate) printf '{"valid":true,"checked":"%s"}' "$PWD" ;;
  compile) printf '{"system":"fixture","source":"%s/system.yaml"}' "$PWD" ;;
  graph)
    if [ "$5" = mermaid ]; then echo "graph TD"; else printf '{"nodes":[],"edges":[]}'; fi ;;
  *) printf '{}' ;;
esac
"#;

/// A stand-in home holding the project (`home/proj`) and the `PATH` directory (`home/bin`).
struct Fixture {
    home: TempDir,
}

impl Fixture {
    fn new() -> Fixture {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("proj");
        let bin = home.path().join("bin");
        fs::create_dir_all(project.join(".engineering")).unwrap();
        fs::create_dir_all(project.join("crates/a/ess")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::write(project.join(".engineering/project.yaml"), "name: fixture\n").unwrap();
        fs::write(project.join("system.yaml"), "x\n").unwrap();
        fs::write(project.join("crates/a/ess/system.yaml"), "x\n").unwrap();
        fs::write(project.join("README.md"), "# Fixture\n").unwrap();
        git(&project, &["init", "--quiet"]);
        git(&project, &["add", "-A"]);
        git(&project, &["commit", "--quiet", "-m", "first"]);
        std::os::unix::fs::symlink(real_tool("git"), bin.join("git")).unwrap();
        stub(&bin, "aep", AEP);
        stub(&bin, "ess", ESS);
        Fixture { home }
    }

    fn home(&self) -> PathBuf {
        fs::canonicalize(self.home.path()).unwrap()
    }

    fn project(&self) -> PathBuf {
        self.home().join("proj")
    }

    fn env(&self) -> Env {
        Env::with_path(self.project(), self.home().join("bin").into_os_string())
    }

    fn options(&self, out: &Path) -> Options {
        Options {
            out: out.to_path_buf(),
            force: false,
            token: TOKEN.to_owned(),
            home: Some(self.home()),
        }
    }
}

fn stub(dir: &Path, name: &str, script: &str) {
    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn site() -> Site {
    Site::new(
        INDEX.as_bytes().to_vec(),
        vec![
            ("index.html".to_owned(), INDEX.as_bytes().to_vec()),
            (
                "assets/index-abc12345.js".to_owned(),
                b"console.log(1)".to_vec(),
            ),
            ("assets/index-abc12345.css".to_owned(), b"body{}".to_vec()),
        ],
    )
}

/// One export of the fixture into `<scratch>/site`.
fn exported() -> (Fixture, TempDir, PathBuf) {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    export::export(&fixture.env(), &site(), &fixture.options(&out)).expect("export succeeds");
    (fixture, scratch, out)
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every regular file under `dir`, relative, `/`-separated.
fn files(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path.strip_prefix(dir).unwrap();
                out.insert(relative.to_str().unwrap().to_owned());
            }
        }
    }
    out
}

// Acceptance 1.

#[test]
fn index_html_is_the_embedded_one_in_static_mode_with_its_assets() {
    let (_fixture, _scratch, out) = exported();
    let index = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        index.contains(r#"<meta name="repoview-mode" content="static" />"#),
        "{index}"
    );
    assert!(!index.contains(r#"content="server""#), "{index}");
    assert!(
        !index.to_ascii_lowercase().contains("<base"),
        "a static copy may live under any path: {index}"
    );
    assert_eq!(
        index.replace(r#"content="static""#, r#"content="server""#),
        INDEX
    );
    assert_eq!(
        fs::read(out.join("assets/index-abc12345.js")).unwrap(),
        b"console.log(1)"
    );
    assert_eq!(
        fs::read(out.join("assets/index-abc12345.css")).unwrap(),
        b"body{}"
    );
}

#[test]
fn an_index_html_without_the_mode_meta_is_refused() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    let bare = Site::new(b"<!doctype html><title>x</title>".to_vec(), Vec::new());
    let error = export::export(&fixture.env(), &bare, &fixture.options(&out)).unwrap_err();
    assert!(error.to_string().contains("repoview-mode"), "{error}");
}

#[test]
fn every_answer_is_one_json_file_at_data_api_path() {
    let (_fixture, _scratch, out) = exported();
    let data: BTreeSet<String> = files(&out.join("data"));
    for expected in [
        "snapshot.json",
        "plan/board.json",
        "plan/artifacts.json",
        "plan/graph.json",
        "plan/artifacts/story:one.json",
        "plan/artifacts/story:one/history.json",
        "plan/artifacts/story:one/explain.json",
        "plan/artifacts/epic:two.json",
        "plan/artifacts/epic:two/history.json",
        "spec/roots.json",
        "spec/roots/~/ir.json",
        "spec/roots/~/graph.json",
        "spec/roots/~/mermaid.json",
        "spec/roots/crates/a/ess/ir.json",
        "spec/roots/crates/a/ess/graph.json",
        "spec/roots/crates/a/ess/mermaid.json",
        "vcs.json",
        "docs.json",
        "docs/README.md.json",
        "export.json",
    ] {
        assert!(
            data.contains(expected),
            "data/{expected} missing from {data:#?}"
        );
    }
    // quality answers whatever the codegate locator finds; either way it is written.
    assert!(
        data.contains("quality.json") || data.contains("quality.error.json"),
        "{data:#?}"
    );
    // Only present documents.
    for absent in ["AGENTS.md", "STATUS.md", "CHANGELOG.md"] {
        assert!(
            !data
                .iter()
                .any(|path| path.starts_with(&format!("docs/{absent}"))),
            "{absent}: {data:#?}"
        );
    }
}

#[test]
fn answers_come_from_the_tools_through_the_router() {
    let (_fixture, _scratch, out) = exported();
    let artifacts = read_json(&out.join("data/plan/artifacts.json"));
    let ids: Vec<&str> = artifacts
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["story:one", "epic:two"]);
    assert_eq!(
        read_json(&out.join("data/spec/roots/crates/a/ess/mermaid.json")),
        serde_json::json!({ "mermaid": "graph TD\n" })
    );
    let roots = read_json(&out.join("data/spec/roots.json"));
    let roots: Vec<&str> = roots
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["root"].as_str().unwrap())
        .collect();
    assert_eq!(roots, [".", "crates/a/ess"]);
}

#[test]
fn a_written_answer_equals_the_routers_own_answer() {
    let (fixture, _scratch, out) = exported();
    let state = AppState {
        token: TOKEN.to_owned(),
        port: 7481,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| serde_json::json!({})),
    };
    let app = router(state).layer(Extension(fixture.env()));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let body = runtime.block_on(async move {
        let request = Request::get("/api/plan/artifacts/story%3Aone/history")
            .header(header::HOST, "127.0.0.1:7481")
            .header(TOKEN_HEADER, TOKEN)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
        to_bytes(response.into_body(), usize::MAX).await.unwrap()
    });
    let direct: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        read_json(&out.join("data/plan/artifacts/story:one/history.json")),
        direct
    );
}

// Acceptance 2.

#[test]
fn a_non_2xx_answer_is_an_error_file_with_status_and_body() {
    let (_fixture, _scratch, out) = exported();
    let data = out.join("data");
    assert!(!data.join("plan/validate.json").exists());
    let validate = read_json(&data.join("plan/validate.error.json"));
    assert_eq!(validate["status"], 502);
    assert_eq!(validate["body"]["tool"], "aep");
    assert_eq!(validate["body"]["exit"], 1);
    assert!(
        validate["body"]["stdout"]
            .as_str()
            .unwrap()
            .contains("story:one has no acceptance"),
        "{validate}"
    );
    assert!(!data.join("plan/artifacts/epic:two/explain.json").exists());
    let explain = read_json(&data.join("plan/artifacts/epic:two/explain.error.json"));
    assert_eq!(explain["status"], 502);
    assert_eq!(explain["body"]["exit"], 3);
}

#[test]
fn export_json_lists_every_route_with_its_status() {
    let (_fixture, _scratch, out) = exported();
    let manifest = read_json(&out.join("data/export.json"));
    assert_eq!(manifest["repoview_version"], env!("CARGO_PKG_VERSION"));
    let at = manifest["exported_at"].as_str().unwrap();
    assert!(
        at.len() == 20 && at.ends_with('Z') && at.as_bytes()[10] == b'T',
        "RFC 3339 UTC: {at}"
    );
    let routes: Vec<(String, u64)> = manifest["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|route| {
            (
                route["path"].as_str().unwrap().to_owned(),
                route["status"].as_u64().unwrap(),
            )
        })
        .collect();
    let listed: BTreeSet<&str> = routes.iter().map(|(path, _)| path.as_str()).collect();
    for (path, status) in [
        ("snapshot", 200),
        ("plan/board", 200),
        ("plan/validate", 502),
        ("plan/artifacts/epic:two/explain", 502),
        ("plan/artifacts/story:one/explain", 200),
        ("spec/roots/~/ir", 200),
        ("docs/README.md", 200),
    ] {
        assert!(
            routes.contains(&(path.to_owned(), status)),
            "{path} {status} not in {routes:?}"
        );
    }
    // Every listed route has its file, and every data file but the manifest is a listed route.
    let data = out.join("data");
    for (path, status) in &routes {
        let file = if (200..300).contains(status) {
            format!("{path}.json")
        } else {
            format!("{path}.error.json")
        };
        assert!(data.join(&file).is_file(), "data/{file}");
    }
    for file in files(&data) {
        if file == "export.json" {
            continue;
        }
        let route = file
            .strip_suffix(".error.json")
            .or_else(|| file.strip_suffix(".json"))
            .unwrap();
        assert!(listed.contains(route), "data/{file} is not a listed route");
    }
}

// Acceptance 3.

#[test]
fn no_file_holds_the_token_or_the_home_directory() {
    let (fixture, _scratch, out) = exported();
    let home = fixture.home();
    let home = home.to_str().unwrap();
    let mut checked = 0;
    for file in files(&out) {
        let text = String::from_utf8_lossy(&fs::read(out.join(&file)).unwrap()).into_owned();
        assert!(!text.contains(TOKEN), "{file} holds the run token");
        assert!(
            !text.contains(home),
            "{file} holds the home path {home}: {text}"
        );
        checked += 1;
    }
    assert!(checked > 20, "only {checked} files checked");
    // The home path did reach the answers before the export rewrote them.
    let snapshot = read_json(&out.join("data/snapshot.json"));
    assert_eq!(snapshot["project"]["root"], ".");
    let show = read_json(&out.join("data/plan/artifacts/story:one.json"));
    assert_eq!(show["path"], "./.engineering/planning/x.md");
    let ir = read_json(&out.join("data/spec/roots/crates/a/ess/ir.json"));
    assert_eq!(ir["source"], "./system.yaml");
}

#[test]
fn every_tool_path_is_the_tools_file_name() {
    let (_fixture, _scratch, out) = exported();
    let mut seen = Vec::new();
    for file in files(&out.join("data")) {
        collect_tool_paths(&read_json(&out.join("data").join(&file)), &mut seen);
    }
    assert!(seen.contains(&"aep".to_owned()), "{seen:?}");
    assert!(seen.contains(&"ess".to_owned()), "{seen:?}");
    assert!(seen.contains(&"git".to_owned()), "{seen:?}");
    for path in &seen {
        assert!(!path.contains('/'), "tool_path {path:?} is not a file name");
    }
}

fn collect_tool_paths(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "tool_path"
                    && let Value::String(path) = value
                {
                    out.push(path.clone());
                }
                collect_tool_paths(value, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_tool_paths(item, out)),
        _ => {}
    }
}

#[test]
fn a_home_path_inside_a_longer_name_is_left_alone() {
    // `<home>x` is not under `<home>`; only whole path components are rewritten.
    let scrub = export::Scrub::new(Path::new("/srv/p"), Some(Path::new("/srv/u")), TOKEN);
    assert_eq!(scrub.text("/srv/u/a and /srv/ux"), "~/a and /srv/ux");
    assert_eq!(scrub.text("/srv/p"), ".");
    assert_eq!(scrub.text("at /srv/p/b: no"), "at ./b: no");
    assert_eq!(scrub.text(&format!("t={TOKEN}")), "t=[token]");
}

// Acceptance 4: every file the pages request exists. The requests are derived from the exported
// answers the way the pages derive them (web/src/api/*.ts), not from the export's own route list.

#[test]
fn every_file_the_pages_request_exists() {
    let (_fixture, _scratch, out) = exported();
    let data = out.join("data");
    let mut requests: Vec<String> = [
        "snapshot",
        "plan/board",
        "plan/artifacts",
        "plan/validate",
        "spec/roots",
        "quality",
        "vcs",
        "docs",
    ]
    .map(str::to_owned)
    .to_vec();
    for item in read_json(&data.join("plan/artifacts.json"))
        .as_array()
        .unwrap()
    {
        let id = item["id"].as_str().unwrap();
        requests.push(format!("plan/artifacts/{id}"));
        requests.push(format!("plan/artifacts/{id}/history"));
        requests.push(format!("plan/artifacts/{id}/explain"));
    }
    for entry in read_json(&data.join("spec/roots.json")).as_array().unwrap() {
        let root = entry["root"].as_str().unwrap();
        let key = if root == "." { "~" } else { root };
        for view in ["ir", "graph", "mermaid"] {
            requests.push(format!("spec/roots/{key}/{view}"));
        }
    }
    for document in read_json(&data.join("docs.json")).as_array().unwrap() {
        if document["present"] == true {
            requests.push(format!("docs/{}", document["name"].as_str().unwrap()));
        }
    }
    // 8 fixed, 3 for each of 2 artifacts, 3 for each of 2 roots, 1 present document.
    assert_eq!(requests.len(), 21, "{requests:?}");
    for request in &requests {
        let answer = data.join(format!("{request}.json"));
        let error = data.join(format!("{request}.error.json"));
        assert!(
            answer.is_file() || error.is_file(),
            "the pages request ./data/{request}.json and nothing answers it"
        );
    }
}

// Acceptance 5.

#[test]
fn an_absent_out_directory_is_created_and_an_empty_one_is_used() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let absent = scratch.path().join("a/b/site");
    export::export(&fixture.env(), &site(), &fixture.options(&absent)).unwrap();
    assert!(absent.join("index.html").is_file());
    let empty = scratch.path().join("empty");
    fs::create_dir(&empty).unwrap();
    export::export(&fixture.env(), &site(), &fixture.options(&empty)).unwrap();
    assert!(empty.join("data/export.json").is_file());
}

#[test]
fn a_non_empty_out_directory_is_refused_without_force() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("mine.txt"), "keep").unwrap();
    let error = export::export(&fixture.env(), &site(), &fixture.options(&out)).unwrap_err();
    assert!(matches!(error, Error::Refused(_)), "{error}");
    assert!(error.to_string().contains("--force"), "{error}");
    assert_eq!(files(&out), BTreeSet::from(["mine.txt".to_owned()]));
}

#[test]
fn force_refuses_a_directory_no_export_wrote() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    fs::create_dir_all(out.join("data")).unwrap();
    fs::write(out.join("index.html"), "mine").unwrap();
    let options = Options {
        force: true,
        ..fixture.options(&out)
    };
    let error = export::export(&fixture.env(), &site(), &options).unwrap_err();
    assert!(matches!(error, Error::Refused(_)), "{error}");
    assert!(error.to_string().contains("data/export.json"), "{error}");
    assert_eq!(fs::read_to_string(out.join("index.html")).unwrap(), "mine");
}

#[test]
fn force_replaces_only_what_a_previous_export_wrote() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    let older = Site::new(
        INDEX.as_bytes().to_vec(),
        vec![
            ("assets/old-11111111.js".to_owned(), b"old".to_vec()),
            ("assets/deep/old-22222222.js".to_owned(), b"old".to_vec()),
        ],
    );
    export::export(&fixture.env(), &older, &fixture.options(&out)).unwrap();
    let manifest = read_json(&out.join("data/export.json"));
    let listed: Vec<&str> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file.as_str().unwrap())
        .collect();
    assert!(listed.contains(&"assets/old-11111111.js"), "{listed:?}");
    assert!(listed.contains(&"data/export.json"), "{listed:?}");
    // Files the export did not write, beside and inside its own directories.
    fs::write(out.join("CNAME"), "docs.example.invalid").unwrap();
    fs::create_dir(out.join(".git")).unwrap();
    fs::write(out.join(".git/HEAD"), "ref").unwrap();
    fs::write(out.join("assets/mine.png"), "png").unwrap();

    let options = Options {
        force: true,
        ..fixture.options(&out)
    };
    export::export(&fixture.env(), &site(), &options).unwrap();
    assert_eq!(
        fs::read_to_string(out.join("CNAME")).unwrap(),
        "docs.example.invalid"
    );
    assert_eq!(fs::read_to_string(out.join(".git/HEAD")).unwrap(), "ref");
    assert_eq!(
        fs::read_to_string(out.join("assets/mine.png")).unwrap(),
        "png"
    );
    assert!(!out.join("assets/old-11111111.js").exists());
    assert!(
        !out.join("assets/deep").exists(),
        "an emptied directory is removed"
    );
    assert!(out.join("assets/index-abc12345.js").is_file());
    assert!(out.join("data/export.json").is_file());
    assert!(out.join("index.html").is_file());
}

// Acceptance 5, symlinks: nothing outside --out is removed or written through a symlink.

#[test]
fn an_out_directory_that_is_a_symlink_is_refused() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let real = scratch.path().join("real");
    fs::create_dir(&real).unwrap();
    let link = scratch.path().join("site");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let error = export::export(&fixture.env(), &site(), &fixture.options(&link)).unwrap_err();
    assert!(matches!(error, Error::Refused(_)), "{error}");
    assert!(error.to_string().contains("symlink"), "{error}");
    assert!(files(&real).is_empty());
}

#[test]
fn a_symlinked_directory_a_write_would_pass_through_is_refused_before_anything_is_removed() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let outside = scratch.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let out = scratch.path().join("site");
    export::export(&fixture.env(), &site(), &fixture.options(&out)).unwrap();
    // `assets` replaced by a symlink out of --out; the next export writes assets/….
    fs::remove_dir_all(out.join("assets")).unwrap();
    std::os::unix::fs::symlink(&outside, out.join("assets")).unwrap();
    let options = Options {
        force: true,
        ..fixture.options(&out)
    };
    let error = export::export(&fixture.env(), &site(), &options).unwrap_err();
    assert!(matches!(error, Error::Refused(_)), "{error}");
    assert!(files(&outside).is_empty(), "{:?}", files(&outside));
    assert!(
        out.join("data/export.json").is_file(),
        "nothing was removed"
    );
    assert!(out.join("index.html").is_file(), "nothing was removed");
}

#[test]
fn every_tool_path_is_cut_to_its_file_name_wherever_it_appears() {
    let scrub = export::Scrub::new(Path::new("/srv/p"), None, TOKEN)
        .tools(["/opt/tools/bin/aep", "/usr/local/bin/codegate"]);
    assert_eq!(
        scrub.text("/opt/tools/bin/aep exited with exit status: 1"),
        "aep exited with exit status: 1"
    );
    assert_eq!(
        scrub.value(serde_json::json!({ "skipped": ["/usr/local/bin/codegate"] })),
        serde_json::json!({ "skipped": ["codegate"] })
    );
}

// The command line.

#[test]
fn cli_refuses_a_non_empty_out_directory_with_exit_2() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("mine.txt"), "keep").unwrap();
    let output = repoview()
        .args(["export", "--out"])
        .arg(&out)
        .arg("--root")
        .arg(fixture.project())
        .env("PATH", fixture.home().join("bin"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--force"), "{stderr}");
    assert_eq!(files(&out), BTreeSet::from(["mine.txt".to_owned()]));
}

#[test]
fn cli_force_on_a_directory_no_export_wrote_exits_2() {
    let fixture = Fixture::new();
    let scratch = tempfile::tempdir().unwrap();
    let out = scratch.path().join("site");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("mine.txt"), "keep").unwrap();
    let output = repoview()
        .args(["export", "--force", "--out"])
        .arg(&out)
        .arg("--root")
        .arg(fixture.project())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(files(&out), BTreeSet::from(["mine.txt".to_owned()]));
}

#[test]
fn cli_help_names_out_root_and_force() {
    let output = repoview().args(["export", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8_lossy(&output.stdout);
    for flag in ["--out <DIR>", "--root <DIR>", "--force"] {
        assert!(help.contains(flag), "{flag}: {help}");
    }
}
