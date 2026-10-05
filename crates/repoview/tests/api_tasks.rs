//! story:taskfile-tasks acceptance 1: `/api/tasks` reads `Taskfile.yml` and the `includes:` files
//! inside the project with a YAML parser, and starts no process.
//!
//! Every case runs with a `PATH` of recording stubs (`task`, `sh`, `bash`, `touch`, `git`), so a
//! process the route starts leaves a `<name>-was-started` file behind.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tempfile::TempDir;

const STUBS: [&str; 5] = ["task", "sh", "bash", "touch", "git"];
const MIB: usize = 1024 * 1024;

fn token() -> String {
    "a".repeat(64)
}

/// An executable `/bin/sh` script `name` in `dir` that records, in a fixed directory, that it was
/// started. Waits out ETXTBSY from a concurrently forked test thread.
fn recording_stub(dir: &Path, name: &str, record: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n: > '{}/{name}-was-started'\n",
            record.to_str().unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    for _ in 0..200 {
        match Command::new(&path).current_dir(scratch.path()).output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = fs::remove_file(record.join(format!("{name}-was-started")));
                return;
            }
        }
    }
    panic!("stub {name} stayed busy");
}

/// A project under a scratch directory, so a case can put files beside it (outside the project).
struct Project {
    _scratch: TempDir,
    root: std::path::PathBuf,
    outside: std::path::PathBuf,
    bin: std::path::PathBuf,
    record: std::path::PathBuf,
}

impl Project {
    fn new() -> Project {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("project");
        let outside = scratch.path().join("outside");
        let bin = scratch.path().join("bin");
        let record = scratch.path().join("record");
        for dir in [&root, &outside, &bin, &record] {
            fs::create_dir(dir).unwrap();
        }
        for name in STUBS {
            recording_stub(&bin, name, &record);
        }
        Project {
            _scratch: scratch,
            root,
            outside,
            bin,
            record,
        }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    async fn get_raw(&self) -> (StatusCode, Vec<u8>) {
        let state = AppState {
            token: token(),
            port: 7480,
            assets: Arc::new(MemoryAssets::new()),
            snapshot: Arc::new(|| json!({ "sources": [] })),
        };
        let response = tower::ServiceExt::oneshot(
            router(state).layer(Extension(Env::with_path(&self.root, &self.bin))),
            Request::get("/api/tasks")
                .header(header::HOST, "127.0.0.1:7480")
                .header(TOKEN_HEADER, token())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, body.to_vec())
    }

    /// `GET /api/tasks`; asserts that no stub was started and the answer is JSON.
    async fn get(&self) -> (StatusCode, Value) {
        let (status, body) = self.get_raw().await;
        self.assert_nothing_started();
        let value = serde_json::from_slice(&body).unwrap_or_else(|error| {
            panic!(
                "/api/tasks answered {status} with non-JSON ({error}): {}",
                String::from_utf8_lossy(&body)
            )
        });
        (status, value)
    }

    async fn ok(&self) -> Value {
        let (status, body) = self.get().await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    fn assert_nothing_started(&self) {
        let started: Vec<_> = fs::read_dir(&self.record)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert!(started.is_empty(), "/api/tasks started {started:?}");
    }
}

fn names(body: &Value) -> Vec<String> {
    body["tasks"]
        .as_array()
        .unwrap_or_else(|| panic!("no tasks array: {body}"))
        .iter()
        .map(|task| task["name"].as_str().unwrap().to_owned())
        .collect()
}

fn task<'a>(body: &'a Value, name: &str) -> &'a Value {
    body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["name"] == name)
        .unwrap_or_else(|| panic!("no task {name}: {body}"))
}

/// The `refused` entry for include `namespace`.
fn refused<'a>(body: &'a Value, namespace: &str) -> &'a Value {
    body["refused"]
        .as_array()
        .unwrap_or_else(|| panic!("no refused array: {body}"))
        .iter()
        .find(|entry| entry["include"] == namespace)
        .unwrap_or_else(|| panic!("include {namespace} not refused: {body}"))
}

// ---- the task list -------------------------------------------------------------------------

#[tokio::test]
async fn tasks_carry_name_desc_summary_internal_and_aliases_in_file_order() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        concat!(
            "version: '3'\n",
            "tasks:\n",
            "  check:\n",
            "    desc: The gate\n",
            "    summary: |\n",
            "      Runs every check.\n",
            "      Twice.\n",
            "    aliases: [c, gate]\n",
            "    cmds: [cargo test]\n",
            "  build:\n",
            "    desc: Build {{.APP}}\n",
            "    internal: true\n",
            "  plain: echo shorthand\n",
            "  listed: [echo one, echo two]\n",
            "  empty:\n",
            "  odd:\n",
            "    desc: [not, a, string]\n",
            "    internal: yes please\n",
            "    aliases: other\n",
        ),
    );
    let body = project.ok().await;
    assert_eq!(body["file"], "Taskfile.yml");
    assert_eq!(
        body["tasks"],
        json!([
            {
                "name": "check",
                "desc": "The gate",
                "summary": "Runs every check.\nTwice.\n",
                "internal": false,
                "aliases": ["c", "gate"],
            },
            {
                "name": "build",
                "desc": "Build {{.APP}}",
                "summary": null,
                "internal": true,
                "aliases": [],
            },
            { "name": "plain", "desc": null, "summary": null, "internal": false, "aliases": [] },
            { "name": "listed", "desc": null, "summary": null, "internal": false, "aliases": [] },
            { "name": "empty", "desc": null, "summary": null, "internal": false, "aliases": [] },
            { "name": "odd", "desc": null, "summary": null, "internal": false, "aliases": [] },
        ])
    );
    assert_eq!(body["refused"], json!([]));
}

#[tokio::test]
async fn a_taskfile_without_tasks_lists_none() {
    let project = Project::new();
    project.write("Taskfile.yml", "version: '3'\n");
    let body = project.ok().await;
    assert_eq!(body["tasks"], json!([]));
}

/// Task's other default names are read when `Taskfile.yml` is absent; `file` names the one read.
#[tokio::test]
async fn another_default_taskfile_name_is_read_and_named() {
    let project = Project::new();
    project.write("taskfile.yaml", "tasks:\n  lint: {desc: Lint}\n");
    let body = project.ok().await;
    assert_eq!(body["file"], "taskfile.yaml");
    assert_eq!(names(&body), ["lint"]);
}

// ---- no process ----------------------------------------------------------------------------

/// The finding that removed `task --list-all --json` in wave two: `task` evaluates `vars: sh:`
/// when the Taskfile declares `dotenv:`. Reading the file runs neither.
#[tokio::test]
async fn sh_variables_and_dotenv_start_no_process() {
    let project = Project::new();
    project.write(".env", "A=1\n");
    project.write(
        "Taskfile.yml",
        concat!(
            "version: '3'\n",
            "dotenv: ['.env']\n",
            "vars:\n",
            "  PROBE:\n",
            "    sh: touch executed-by-listing\n",
            "includes:\n",
            "  sub: ./sub\n",
            "tasks:\n",
            "  check:\n",
            "    desc: '{{.PROBE}}'\n",
            "    vars:\n",
            "      INNER: {sh: touch executed-by-task-var}\n",
            "    cmds: [touch executed-by-cmd]\n",
        ),
    );
    project.write(
        "sub/Taskfile.yml",
        concat!(
            "dotenv: ['../.env']\n",
            "vars: {SUB: {sh: touch executed-by-include}}\n",
            "tasks:\n  t: {desc: '{{.SUB}}'}\n",
        ),
    );
    let body = project.ok().await;
    assert_eq!(names(&body), ["check", "sub:t"]);
    assert_eq!(task(&body, "check")["desc"], "{{.PROBE}}");
    for probe in [
        "executed-by-listing",
        "executed-by-task-var",
        "executed-by-cmd",
        "executed-by-include",
        "sub/executed-by-include",
    ] {
        assert!(
            !project.root.join(probe).exists(),
            "GET /api/tasks created {probe}"
        );
    }
}

// ---- absent and unreadable -----------------------------------------------------------------

#[tokio::test]
async fn no_taskfile_is_absent() {
    let project = Project::new();
    project.write("README.md", "# x\n");
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["availability"], "Absent", "{body}");
}

#[tokio::test]
async fn a_directory_named_taskfile_is_absent() {
    let project = Project::new();
    fs::create_dir(project.root.join("Taskfile.yml")).unwrap();
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

/// A FIFO named `Taskfile.yml` must neither block the request nor be read.
#[tokio::test]
async fn a_fifo_named_taskfile_does_not_block() {
    let project = Project::new();
    let fifo = std::ffi::CString::new(project.root.join("Taskfile.yml").to_str().unwrap()).unwrap();
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    let (status, _) = tokio::time::timeout(Duration::from_secs(10), project.get())
        .await
        .expect("GET /api/tasks blocked on a FIFO");
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_yaml_is_failed_with_a_diagnostic() {
    let project = Project::new();
    project.write("Taskfile.yml", "tasks:\n  a: [\n");
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["availability"], "Failed", "{body}");
    assert!(
        body["diagnostic"]
            .as_str()
            .unwrap()
            .contains("Taskfile.yml")
    );
}

#[tokio::test]
async fn a_taskfile_that_is_not_a_mapping_is_failed() {
    let project = Project::new();
    project.write("Taskfile.yml", "- just\n- a list\n");
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["availability"], "Failed", "{body}");
}

/// Exactly 1 MiB is read; one byte more is refused before parsing.
#[tokio::test]
async fn a_taskfile_over_one_mebibyte_is_failed() {
    let project = Project::new();
    let head = "tasks:\n  a: {desc: A}\n#";
    let mut text = String::from(head);
    text.push_str(&"x".repeat(MIB - head.len() - 1));
    text.push('\n');
    assert_eq!(text.len(), MIB);
    project.write("Taskfile.yml", &text);
    assert_eq!(names(&project.ok().await), ["a"]);

    text.insert(0, ' ');
    project.write("Taskfile.yml", &text);
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(
        body["diagnostic"].as_str().unwrap().contains("1 MiB"),
        "{body}"
    );
}

/// Nested aliases that expand exponentially are refused by the parser, quickly.
#[tokio::test]
async fn an_alias_bomb_fails_quickly() {
    let project = Project::new();
    let mut text = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
    for level in 1..10 {
        let prev = level - 1;
        text.push_str(&format!(
            "a{level}: &a{level} [*a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}]\n"
        ));
    }
    text.push_str("tasks:\n  t:\n    desc: d\n    aliases: *a9\n");
    project.write("Taskfile.yml", &text);
    let started = Instant::now();
    let (status, body) = project.get().await;
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
}

/// The root Taskfile may be a symlink inside the project, never one that leaves it.
#[tokio::test]
async fn a_root_taskfile_symlinked_out_of_the_project_is_not_read() {
    let project = Project::new();
    fs::write(
        project.outside.join("Taskfile.yml"),
        "tasks:\n  secret: {desc: outside}\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        project.outside.join("Taskfile.yml"),
        project.root.join("Taskfile.yml"),
    )
    .unwrap();
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(!body.to_string().contains("secret"), "{body}");

    fs::remove_file(project.root.join("Taskfile.yml")).unwrap();
    project.write("tasks/main.yml", "tasks:\n  inside: {desc: in}\n");
    std::os::unix::fs::symlink("tasks/main.yml", project.root.join("Taskfile.yml")).unwrap();
    assert_eq!(names(&project.ok().await), ["inside"]);
}

/// A symlink into `.git` stays inside the project, but `.git` is not the project's source.
#[tokio::test]
async fn a_taskfile_symlinked_into_dot_git_is_not_read() {
    let project = Project::new();
    project.write(".git/config", "tasks:\n  leaked: {desc: from .git}\n");
    std::os::unix::fs::symlink(".git/config", project.root.join("Taskfile.yml")).unwrap();
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(!body.to_string().contains("leaked"), "{body}");
}

// ---- includes ------------------------------------------------------------------------------

/// The string form, the mapping form (`taskfile`, `internal`, `flatten`, `excludes`), a directory
/// include and an optional include that is missing. A task's aliases carry its namespace; an
/// include's own `aliases` (namespace aliases) are not expanded.
#[tokio::test]
async fn includes_inside_the_project_are_namespaced() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        concat!(
            "includes:\n",
            "  docs: ./docs/Taskfile.yml\n",
            "  web:\n",
            "    taskfile: web\n",
            "    aliases: [w]\n",
            "  lib:\n",
            "    taskfile: lib/tasks.yml\n",
            "    internal: true\n",
            "    excludes: [skipped]\n",
            "  flat:\n",
            "    taskfile: flat.yml\n",
            "    flatten: true\n",
            "  maybe:\n",
            "    taskfile: missing.yml\n",
            "    optional: true\n",
            "tasks:\n",
            "  root: {desc: Root}\n",
        ),
    );
    project.write("docs/Taskfile.yml", "tasks:\n  build: {desc: Docs}\n");
    project.write(
        "web/Taskfile.yml",
        "tasks:\n  dev: {desc: Dev server, aliases: [d]}\n",
    );
    project.write(
        "lib/tasks.yml",
        "tasks:\n  gen: {desc: Generate}\n  skipped: {desc: no}\n",
    );
    project.write("flat.yml", "tasks:\n  fmt: {desc: Format}\n");
    let body = project.ok().await;
    assert_eq!(
        names(&body),
        ["root", "docs:build", "web:dev", "lib:gen", "fmt"]
    );
    assert_eq!(task(&body, "docs:build")["desc"], "Docs");
    assert_eq!(task(&body, "docs:build")["internal"], false);
    assert_eq!(task(&body, "web:dev")["aliases"], json!(["web:d"]));
    assert_eq!(task(&body, "lib:gen")["internal"], true);
    assert_eq!(body["refused"], json!([]));
}

#[tokio::test]
async fn a_missing_required_include_is_refused_and_the_rest_still_lists() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        "includes:\n  gone: ./gone.yml\ntasks:\n  a: {desc: A}\n",
    );
    let body = project.ok().await;
    assert_eq!(names(&body), ["a"]);
    let entry = refused(&body, "gone");
    assert_eq!(entry["taskfile"], "./gone.yml");
    assert!(
        entry["reason"].as_str().unwrap().contains("not found"),
        "{entry}"
    );
}

/// Absolute paths, `..`, remote URLs, home-relative and templated paths are refused without
/// being read; a symlink that leaves the project is refused after resolving it.
#[tokio::test]
async fn includes_that_leave_the_project_are_refused() {
    let project = Project::new();
    fs::write(
        project.outside.join("Taskfile.yml"),
        "tasks:\n  escaped: {desc: outside}\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&project.outside, project.root.join("link-out")).unwrap();
    std::os::unix::fs::symlink(
        project.outside.join("Taskfile.yml"),
        project.root.join("file-out.yml"),
    )
    .unwrap();
    let absolute = project.outside.join("Taskfile.yml");
    project.write(
        "Taskfile.yml",
        &format!(
            concat!(
                "includes:\n",
                "  abs: '{}'\n",
                "  up: ../outside/Taskfile.yml\n",
                "  sneaky: sub/../../outside\n",
                "  remote: https://example.invalid/Taskfile.yml\n",
                "  git: git@example.invalid:o/r.git//Taskfile.yml\n",
                "  home: ~/Taskfile.yml\n",
                "  templated: '{{{{.ROOT_DIR}}}}/../outside'\n",
                "  dirlink: link-out\n",
                "  filelink:\n",
                "    taskfile: file-out.yml\n",
                "tasks:\n",
                "  ok: {{desc: fine}}\n",
            ),
            absolute.to_str().unwrap()
        ),
    );
    let body = project.ok().await;
    assert_eq!(names(&body), ["ok"]);
    assert!(!body.to_string().contains("escaped"), "{body}");
    for namespace in [
        "abs",
        "up",
        "sneaky",
        "remote",
        "git",
        "home",
        "templated",
        "dirlink",
        "filelink",
    ] {
        refused(&body, namespace);
    }
    assert_eq!(body["refused"].as_array().unwrap().len(), 9, "{body}");
}

/// Includes nest at most four deep: levels 1–4 are listed, level 5 is refused.
#[tokio::test]
async fn includes_nest_at_most_four_deep() {
    let project = Project::new();
    project.write("Taskfile.yml", "includes: {n1: l1.yml}\ntasks: {t0: {}}\n");
    for level in 1..=5 {
        let next = level + 1;
        project.write(
            &format!("l{level}.yml"),
            &format!("includes: {{n{next}: l{next}.yml}}\ntasks: {{t{level}: {{}}}}\n"),
        );
    }
    let body = project.ok().await;
    assert_eq!(
        names(&body),
        ["t0", "n1:t1", "n1:n2:t2", "n1:n2:n3:t3", "n1:n2:n3:n4:t4"]
    );
    let entry = refused(&body, "n1:n2:n3:n4:n5");
    assert!(
        entry["reason"].as_str().unwrap().contains("depth"),
        "{entry}"
    );
}

#[tokio::test]
async fn an_include_cycle_is_refused() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        "includes: {self: ./Taskfile.yml, a: a.yml}\ntasks: {r: {}}\n",
    );
    project.write(
        "a.yml",
        "includes: {back: ./Taskfile.yml}\ntasks: {x: {}}\n",
    );
    let body = project.ok().await;
    assert_eq!(names(&body), ["r", "a:x"]);
    refused(&body, "self");
    refused(&body, "a:back");
}

/// Wide fan-out within the depth limit is bounded: each file is parsed once, and the response
/// stops at 10,000 entries with `truncated`. Unbounded, this tree of five files lists
/// 1 + 40 + 40² + 40³ + 40⁴ (about 2.6 million) tasks.
#[tokio::test]
async fn include_fan_out_is_truncated_at_ten_thousand_entries() {
    let project = Project::new();
    let fan_out = |file: &str| -> String { (0..40).map(|i| format!("  i{i}: {file}\n")).collect() };
    project.write(
        "Taskfile.yml",
        &format!("includes:\n{}tasks: {{r: {{}}}}\n", fan_out("w1.yml")),
    );
    for level in 1..=4 {
        let includes = if level < 4 {
            format!("includes:\n{}", fan_out(&format!("w{}.yml", level + 1)))
        } else {
            String::new()
        };
        project.write(
            &format!("w{level}.yml"),
            &format!("{includes}tasks: {{w: {{}}}}\n"),
        );
    }
    let started = Instant::now();
    let body = project.ok().await;
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
    let entries = names(&body).len() + body["refused"].as_array().unwrap().len();
    assert_eq!(entries, 10_000, "{entries} entries");
    assert_eq!(body["truncated"], true);
}

/// A response under the entry limit is not truncated.
#[tokio::test]
async fn a_small_taskfile_is_not_truncated() {
    let project = Project::new();
    project.write("Taskfile.yml", "tasks: {a: {}}\n");
    assert_eq!(project.ok().await["truncated"], false);
}

/// Every distinct file read counts against one budget of 128 Taskfiles, the root included.
#[tokio::test]
async fn distinct_included_files_are_bounded_at_128() {
    let project = Project::new();
    let includes: String = (0..140).map(|i| format!("  i{i}: f{i}.yml\n")).collect();
    project.write("Taskfile.yml", &format!("includes:\n{includes}"));
    for i in 0..140 {
        project.write(&format!("f{i}.yml"), "tasks: {t: {}}\n");
    }
    let body = project.ok().await;
    assert_eq!(names(&body).len(), 127);
    assert_eq!(names(&body)[126], "i126:t");
    for i in 127..140 {
        let entry = refused(&body, &format!("i{i}"));
        assert!(
            entry["reason"].as_str().unwrap().contains("limit"),
            "{entry}"
        );
    }
}

/// One file included many times is parsed once and does not spend the file budget again.
#[tokio::test]
async fn a_file_included_twice_is_read_once() {
    let project = Project::new();
    let includes: String = (0..200).map(|i| format!("  i{i}: shared.yml\n")).collect();
    project.write("Taskfile.yml", &format!("includes:\n{includes}"));
    project.write("shared.yml", "tasks: {s: {}}\n");
    let body = project.ok().await;
    assert_eq!(names(&body).len(), 200);
    assert_eq!(body["refused"], json!([]));
}

/// Merge keys apply after the node guard, at any depth, as `task` applies them.
#[tokio::test]
async fn merge_keys_supply_task_fields() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        concat!(
            "x-a: &a {desc: From a, internal: true}\n",
            "x-b: &b {summary: From b}\n",
            "tasks:\n",
            "  one: {<<: *a}\n",
            "  two:\n",
            "    <<: [*a, *b]\n",
            "    desc: Own\n",
        ),
    );
    let body = project.ok().await;
    assert_eq!(task(&body, "one")["desc"], "From a");
    assert_eq!(task(&body, "one")["internal"], true);
    assert_eq!(task(&body, "two")["desc"], "Own");
    assert_eq!(task(&body, "two")["summary"], "From b");
}

/// The node budget is shared by every Taskfile of a request: an include that would pass it is
/// refused, and the root's tasks still list.
#[tokio::test]
async fn the_node_budget_spans_includes() {
    let project = Project::new();
    project.write("Taskfile.yml", "includes: {big: big.yml}\ntasks: {r: {}}\n");
    let mut text = String::from("x-wide: &w [");
    text.push_str(&vec!["x"; 1000].join(", "));
    text.push_str("]\nx-fan: [");
    text.push_str(&vec!["*w"; 300].join(", "));
    text.push_str("]\n");
    project.write("big.yml", &text);
    let started = Instant::now();
    let body = project.ok().await;
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(names(&body), ["r"]);
    assert!(
        refused(&body, "big")["reason"]
            .as_str()
            .unwrap()
            .contains("200000 YAML nodes"),
        "{body}"
    );
}

/// Block nesting is pre-scanned too: compact `- ` entries and growing indentation.
#[tokio::test]
async fn deep_block_nesting_is_refused_before_parsing() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        &format!("tasks:\n  t:\n    cmds:\n      {}x\n", "- ".repeat(200)),
    );
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(
        body["diagnostic"].as_str().unwrap().contains("nested"),
        "{body}"
    );

    let mut text = String::from("a:\n");
    for level in 1..200 {
        text.push_str(&format!("{}k{level}:\n", " ".repeat(level)));
    }
    project.write("Taskfile.yml", &text);
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(
        body["diagnostic"].as_str().unwrap().contains("nested"),
        "{body}"
    );
}

/// Brackets that close again, and nesting up to the limit, are read.
#[tokio::test]
async fn nesting_within_the_limit_is_read() {
    let project = Project::new();
    let text = format!(
        "x: {}{}\ntasks:\n  t:\n    desc: '[[[ in a string ]]]'\n    cmds: [\"[ -f x ] && echo {{a: [b]}}\"]\n",
        "[".repeat(100),
        "]".repeat(100)
    );
    project.write("Taskfile.yml", &text);
    assert_eq!(names(&project.ok().await), ["t"]);
}

#[tokio::test]
async fn an_include_over_one_mebibyte_or_invalid_is_refused() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        "includes: {big: big.yml, bad: bad.yml}\ntasks: {r: {}}\n",
    );
    project.write(
        "big.yml",
        &format!("tasks: {{b: {{}}}}\n#{}\n", "x".repeat(MIB)),
    );
    project.write("bad.yml", "tasks: [\n");
    let body = project.ok().await;
    assert_eq!(names(&body), ["r"]);
    assert!(
        refused(&body, "big")["reason"]
            .as_str()
            .unwrap()
            .contains("1 MiB")
    );
    refused(&body, "bad");
}
