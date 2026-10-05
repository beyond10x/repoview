//! Adversary cases for story:taskfile-tasks: YAML alias expansion that the parser's repetition
//! limit does not count, include fan-out over one large file, and merge keys.

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

/// The nested bomb in `api_tasks.rs` is refused because serde_yaml_ng counts alias *jumps*
/// against 100 × the event count. A flat expansion uses one jump per alias: an anchored sequence
/// of N scalars referenced M times is M jumps (under the limit) and N × M nodes. Here N = M = 3000:
/// a 21 KB Taskfile becomes nine million YAML nodes before a single key is looked at; at
/// N = M = 30000 (210 KB, far under the 1 MiB cap) it is nine hundred million.
#[tokio::test]
async fn a_flat_alias_expansion_is_refused_like_the_nested_bomb() {
    const N: usize = 3000;
    const M: usize = 3000;
    let project = Project::new();
    let mut text = String::from("x-wide: &w [");
    text.push_str(&vec!["x"; N].join(", "));
    text.push_str("]\nx-fan: [");
    text.push_str(&vec!["*w"; M].join(", "));
    text.push_str("]\ntasks:\n  t:\n    desc: d\n");
    project.write("Taskfile.yml", &text);
    assert!(project.bytes_on_disk() < 32 * 1024);
    let started = Instant::now();
    let (status, body) = project.get().await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "a {N}×{M} alias expansion was parsed in full ({:?}) and answered {body}",
        started.elapsed()
    );
}

/// Every read counts against the 128-file budget, but nothing bounds what one read yields: one
/// file of T tasks included K times under different namespaces lists K × T tasks from two files
/// on disk. At the caps (127 includes of a 1 MiB file of ~150 000 tasks) that is ~19 million task
/// objects in one response.
#[tokio::test]
async fn one_file_included_many_times_does_not_multiply_without_bound() {
    const TASKS: usize = 80_000;
    const INCLUDES: usize = 32;
    let project = Project::new();
    let tasks: String = (0..TASKS).map(|i| format!(" t{i}: {{}}\n")).collect();
    project.write("many.yml", &format!("tasks:\n{tasks}"));
    let includes: String = (0..INCLUDES)
        .map(|i| format!("  i{i}: many.yml\n"))
        .collect();
    project.write("Taskfile.yml", &format!("includes:\n{includes}"));
    let on_disk = project.bytes_on_disk();
    assert!(on_disk < 2 * 1024 * 1024, "{on_disk} bytes");
    let started = Instant::now();
    let (status, body) = project.get().await;
    let elapsed = started.elapsed();
    assert_eq!(status, StatusCode::OK);
    assert!(
        (body.len() as u64) < 64 * on_disk && elapsed < Duration::from_secs(10),
        "{on_disk} bytes on disk answered {} bytes in {elapsed:?}",
        body.len()
    );
}

/// `task` decodes each task mapping with yaml.v3 `node.Decode`, which applies `<<` merge keys;
/// serde_yaml_ng's `Value` keeps `<<` as an ordinary key unless `apply_merge` is called.
#[tokio::test]
async fn a_merge_key_supplies_the_desc_as_task_does() {
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        concat!(
            "version: '3'\n",
            "x-defaults: &defaults\n",
            "  desc: Shared description\n",
            "tasks:\n",
            "  build:\n",
            "    <<: *defaults\n",
            "    cmds: [echo build]\n",
        ),
    );
    let (status, body) = project.get().await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tasks"][0]["name"], "build", "{body}");
    assert_eq!(body["tasks"][0]["desc"], "Shared description", "{body}");
}

/// Nesting deeper than serde_yaml_ng's recursion limit (128) is refused only after libyaml has
/// scanned the whole document, and the scanner's flow-level bookkeeping is quadratic in the
/// depth. A Taskfile of `[` × DEPTH then `]` × DEPTH, well under the 1 MiB cap, holds the request
/// thread for longer than the bound the unit set for its own alias bomb.
#[tokio::test]
async fn deep_flow_nesting_is_refused_quickly() {
    const DEPTH: usize = 40_000;
    let project = Project::new();
    project.write(
        "Taskfile.yml",
        &format!("tasks: {}{}\n", "[".repeat(DEPTH), "]".repeat(DEPTH)),
    );
    assert!(project.bytes_on_disk() < 128 * 1024);
    let started = Instant::now();
    let (status, _) = project.get().await;
    let elapsed = started.elapsed();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        elapsed < Duration::from_secs(10),
        "{DEPTH}-deep nesting took {elapsed:?}"
    );
}
