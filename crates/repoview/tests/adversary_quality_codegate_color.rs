//! Adversary case for story:quality-codegate acceptance 2: `commands` is read from
//! `codegate --help`, and codegate is a clap 4 program, which colours its help whenever
//! `CLICOLOR_FORCE` is set (and `NO_COLOR` is not), piped or not. `repoview open` hands its own
//! environment to every `codegate` it runs, so a user with `CLICOLOR_FORCE=1` in their shell must
//! still see codegate 0.3.0's `evaluate` and the story's reason. Its own test binary, because it
//! sets a process environment variable.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use common::bin_dir;
use repoview::assets::MemoryAssets;
use repoview::server::{AppState, TOKEN_HEADER, new_token, router};
use repoview_sources::Env;
use serde_json::{Value, json};
use tower::ServiceExt;

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

/// codegate 0.3.0's `--help` with clap's colour rule: plain under a non-empty `NO_COLOR`,
/// coloured under a non-empty `CLICOLOR_FORCE` other than `0`, plain otherwise. The coloured text
/// is byte for byte what `CLICOLOR_FORCE=1 codegate --help | cat -v` prints for 0.3.0.
const CODEGATE_0_3_0: &str = r#"[ "$1" = probe-busy ] && exit 0
case "$1" in
  --version) printf 'codegate 0.3.0\n'; exit 0 ;;
  --help) ;;
  *) exit 64 ;;
esac
if [ -z "$NO_COLOR" ] && [ -n "$CLICOLOR_FORCE" ] && [ "$CLICOLOR_FORCE" != 0 ]; then
  printf 'Evaluate normalized dependency facts against a language-neutral policy\n\n\033[1m\033[4mUsage:\033[0m \033[1mcodegate\033[0m <COMMAND>\n\n\033[1m\033[4mCommands:\033[0m\n  \033[1mevaluate\033[0m  \n  \033[1mhelp\033[0m      Print this message or the help of the given subcommand(s)\n\n\033[1m\033[4mOptions:\033[0m\n  \033[1m-h\033[0m, \033[1m--help\033[0m     Print help\n  \033[1m-V\033[0m, \033[1m--version\033[0m  Print version\n'
else
  printf 'Evaluate normalized dependency facts against a language-neutral policy\n\nUsage: codegate <COMMAND>\n\nCommands:\n  evaluate  \n  help      Print this message or the help of the given subcommand(s)\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n'
fi"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_user_with_clicolor_force_still_gets_evaluate_and_the_story_reason() {
    // SAFETY: the only test in this binary, set before anything in it reads the environment or
    // spawns a child.
    unsafe {
        std::env::set_var("CLICOLOR_FORCE", "1");
        std::env::remove_var("NO_COLOR");
    }
    let bin = bin_dir(&[]);
    write_stub(bin.path(), "codegate", CODEGATE_0_3_0);
    let dir = tempfile::tempdir().unwrap();
    let token = new_token();
    let app = router(AppState {
        token: token.clone(),
        port: 7480,
        assets: Arc::new(MemoryAssets::new()),
        snapshot: Arc::new(|| json!({ "sources": [] })),
    })
    .layer(Extension(Env::with_path(dir.path(), bin.path())));

    let request = Request::get("/api/quality")
        .header(header::HOST, "127.0.0.1:7480")
        .header(TOKEN_HEADER, &token)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        (body["commands"].clone(), body["reason"].clone()),
        (
            json!(["evaluate"]),
            json!(
                "codegate 0.3.0 evaluates supplied dependency facts only; it has no source \
                 assessment yet"
            )
        ),
        "{body}"
    );
}
