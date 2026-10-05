//! Adversary cases for story:taskfile-tasks, pass 2, against the corrections of round 1: the
//! nesting pre-scan, the node budget, the parse cache and the entry cap.

use std::fs;
use std::path::{Path, PathBuf};
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

fn token() -> String {
    "a".repeat(64)
}

/// A project with an empty `PATH` directory beside it: nothing on it can start.
struct Project {
    _scratch: TempDir,
    root: PathBuf,
    bin: PathBuf,
}

impl Project {
    fn new() -> Project {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("project");
        let bin = scratch.path().join("bin");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&bin).unwrap();
        Project {
            _scratch: scratch,
            root,
            bin,
        }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn bytes_on_disk(&self) -> u64 {
        fn walk(dir: &Path) -> u64 {
            fs::read_dir(dir)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    let meta = entry.metadata().unwrap();
                    if meta.is_dir() {
                        walk(&entry.path())
                    } else {
                        meta.len()
                    }
                })
                .sum()
        }
        walk(&self.root)
    }

    async fn get(&self) -> (StatusCode, Vec<u8>) {
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
}

/// The pre-scan counts every `]` byte as a close, quoted or not; libyaml does not. Each level
/// `["]", ` opens a real flow sequence and holds a one-character string `]`, so the pre-scan's
/// depth never passes 1 while libyaml's grows to DEPTH. This is pass 1's 40,000-deep document
/// (40.5 s) with a quoted `]` beside each `[`, 280 KB on disk.
#[tokio::test]
async fn flow_nesting_hidden_by_quoted_closers_is_refused_quickly() {
    const DEPTH: usize = 40_000;
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        &format!("tasks: {}{}\n", "[\"]\", ".repeat(DEPTH), "]".repeat(DEPTH)),
    );
    assert!(project.bytes_on_disk() < 512 * 1024);
    let started = Instant::now();
    let (status, _) = project.get().await;
    let elapsed = started.elapsed();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        elapsed < Duration::from_secs(10),
        "{DEPTH}-deep nesting behind quoted closers took {elapsed:?}"
    );
}

/// The entry cap counts tasks and refusals, so includes that list nothing never reach it. This
/// is the tree of `include_fan_out_is_truncated_at_ten_thousand_entries` (five files, 40 includes
/// each, four deep) without its tasks: 40 + 40² + 40³ + 40⁴ ≈ 2.6 million include visits, each
/// resolving paths on disk, and an empty answer at the end.
#[tokio::test]
async fn includes_that_list_nothing_are_bounded_too() {
    let project = Project::new();
    let fan_out = |file: &str| -> String { (0..40).map(|i| format!("  i{i}: {file}\n")).collect() };
    project.write("Taskfile.yml", &format!("includes:\n{}", fan_out("w1.yml")));
    for level in 1..=4 {
        let text = if level < 4 {
            format!("includes:\n{}", fan_out(&format!("w{}.yml", level + 1)))
        } else {
            "tasks: {}\n".to_owned()
        };
        project.write(&format!("w{level}.yml"), &text);
    }
    let started = Instant::now();
    let (status, _) = project.get().await;
    let elapsed = started.elapsed();
    assert_eq!(status, StatusCode::OK);
    assert!(
        elapsed < Duration::from_secs(10),
        "an include tree that lists nothing took {elapsed:?}"
    );
}

/// The entry cap bounds how many tasks one response carries, not how large each is: one file
/// with one task whose `desc` is 100 KB, included 1,000 times from the root, is parsed once and
/// copied into the response 1,000 times. At the limits (10,000 entries of a ~1 MiB `desc`) that
/// is a 10 GiB response from two files on disk.
#[tokio::test]
async fn a_large_description_included_many_times_does_not_multiply_the_response() {
    const DESC: usize = 100 * 1024;
    const INCLUDES: usize = 1000;
    let project = Project::new();
    project.write(
        "fat.yml",
        &format!("tasks:\n  t:\n    desc: {}\n", "d".repeat(DESC)),
    );
    let includes: String = (0..INCLUDES)
        .map(|i| format!("  i{i}: fat.yml\n"))
        .collect();
    project.write("Taskfile.yml", &format!("includes:\n{includes}"));
    let on_disk = project.bytes_on_disk();
    let (status, body) = project.get().await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        (body.len() as u64) < 64 * on_disk,
        "{on_disk} bytes on disk answered {} bytes",
        body.len()
    );
}

/// The node budget charges a scalar one node whatever its length, and an alias copies the whole
/// scalar at every use. A 32 KB anchored string referenced 4,000 times is 4,001 nodes, far under
/// 200,000, and 128 MB built from a 48 KB file. At the 1 MiB cap (a 512 KB string referenced
/// ~175,000 times, still under the node budget) it is ~89 GB.
#[tokio::test]
async fn an_aliased_long_scalar_is_refused_like_the_flat_expansion() {
    const SCALAR: usize = 32 * 1024;
    const USES: usize = 4000;
    let project = Project::new();
    let mut text = format!("x-long: &s {}\nx-fan: [", "s".repeat(SCALAR));
    text.push_str(&vec!["*s"; USES].join(","));
    text.push_str("]\ntasks:\n  t:\n    desc: d\n");
    project.write("Taskfile.yml", &text);
    assert!(project.bytes_on_disk() < 64 * 1024);
    let (status, body) = project.get().await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{} bytes of aliased scalar were built and answered {body}",
        SCALAR * USES
    );
}

/// Once the node budget is spent, every later Taskfile is still read and loaded by libyaml in
/// full, only to be refused at its first node. Here the first include spends the whole budget and
/// the other 39 (1 MiB of `a, ` each) are loaded for nothing. 127 such files took 78 s in a debug
/// build; 127 files of 1 MiB comments, which the budget admits, took 8 s.
#[tokio::test]
async fn taskfiles_after_the_node_budget_is_spent_are_refused_without_parsing() {
    const FILES: usize = 40;
    let project = Project::new();
    let includes: String = (0..FILES).map(|i| format!("  i{i}: f{i}.yml\n")).collect();
    project.write(
        "Taskfile.yml",
        &format!("includes:\n{includes}tasks: {{r: {{}}}}\n"),
    );
    let list = "a, ".repeat(349_000);
    for i in 0..FILES {
        project.write(
            &format!("f{i}.yml"),
            &format!("x: [{list}]\ntasks: {{t: {{}}}}\n"),
        );
    }
    let started = Instant::now();
    let (status, body) = project.get().await;
    let elapsed = started.elapsed();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["refused"].as_array().unwrap().len(), FILES, "{body}");
    assert!(
        elapsed < Duration::from_secs(10),
        "{FILES} includes refused by a spent node budget took {elapsed:?}"
    );
}

/// The flow counter is never reset at the end of a line, so every unclosed `[` inside a string
/// adds up over the whole file. ANSI colour codes in `echo` commands (`\033[32m` … `\033[0m`)
/// open two each and close none: 65 coloured echoes refuse the Taskfile as "nested more than
/// 128 levels deep", though its deepest real nesting is four.
#[tokio::test]
async fn unclosed_brackets_inside_strings_do_not_add_up_to_nesting() {
    let project = Project::new();
    let mut text = String::from("version: '3'\ntasks:\n  release:\n    desc: Release\n    cmds:\n");
    for step in 0..65 {
        text.push_str(&format!(
            "      - echo -e \"\\033[32mstep {step} done\\033[0m\"\n"
        ));
    }
    project.write("Taskfile.yml", &text);
    let (status, body) = project.get().await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tasks"][0]["name"], "release", "{body}");
}
