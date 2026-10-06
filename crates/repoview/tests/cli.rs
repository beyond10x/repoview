//! The `repoview` binary, driven as a process: discovery, snapshot, open, doctor, the CLI surface.

mod common;

use std::path::Path;

use common::{Server, bin_dir, canonical, git, git_repo_with_commit, parse_url_line, repoview};
use serde_json::{Value, json};

fn snapshot_json(cwd: &Path, args: &[&str]) -> Value {
    let output = repoview()
        .args(["snapshot", "--format", "json"])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

// Acceptance 2: discovery.

#[test]
fn discovery_from_a_git_subdirectory_is_the_top_level() {
    let repo = git_repo_with_commit();
    let sub = repo.path().join("a/b");
    std::fs::create_dir_all(&sub).unwrap();
    let snapshot = snapshot_json(&sub, &[]);
    assert_eq!(snapshot["project"]["root"], json!(canonical(repo.path())));
}

#[test]
fn discovery_outside_git_is_the_working_directory() {
    let outer = tempfile::tempdir().unwrap();
    let dir = outer.path().join("project");
    std::fs::create_dir(&dir).unwrap();
    let output = repoview()
        .args(["snapshot", "--format", "json"])
        .current_dir(&dir)
        .env("GIT_CEILING_DIRECTORIES", outer.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let snapshot: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot["project"]["root"], json!(canonical(&dir)));
    assert_eq!(snapshot["project"]["name"], json!("project"));
}

#[test]
fn root_flag_overrides_discovery() {
    let repo = git_repo_with_commit();
    let sub = repo.path().join("a");
    std::fs::create_dir_all(&sub).unwrap();
    let other = tempfile::tempdir().unwrap();
    let snapshot = snapshot_json(&sub, &["--root", other.path().to_str().unwrap()]);
    assert_eq!(snapshot["project"]["root"], json!(canonical(other.path())));
}

// Acceptance 3: snapshot of a fresh repository.

#[test]
fn snapshot_of_a_fresh_repository_has_five_sources() {
    let repo = git_repo_with_commit();
    let path = bin_dir(&["git"]);
    let output = repoview()
        .args(["snapshot", "--format", "json", "--root"])
        .arg(repo.path())
        .env("PATH", path.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let snapshot: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        snapshot["repoview_version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
    let sources = snapshot["sources"].as_array().unwrap();
    let ids: Vec<_> = sources.iter().map(|s| s["source_id"].clone()).collect();
    assert_eq!(
        ids,
        vec![
            json!("vcs"),
            json!("plan"),
            json!("spec"),
            json!("quality"),
            json!("docs")
        ]
    );
    let by_id = |id: &str| {
        sources
            .iter()
            .find(|s| s["source_id"] == json!(id))
            .unwrap()
    };
    let vcs = by_id("vcs");
    assert_eq!(vcs["availability"], json!("Present"));
    assert_eq!(
        vcs["summary"]["branch"],
        json!(git(repo.path(), &["branch", "--show-current"]))
    );
    assert_eq!(
        vcs["summary"]["head"],
        json!(git(repo.path(), &["rev-parse", "HEAD"]))
    );
    for id in ["plan", "spec", "docs", "quality"] {
        assert_eq!(by_id(id)["availability"], json!("Absent"), "{id}");
    }
}

// Acceptance 5: open.

fn open_server(root: &Path) -> Server {
    Server::start(&[
        "open",
        "--no-browser",
        "--port",
        "0",
        "--root",
        root.to_str().unwrap(),
    ])
}

#[test]
fn open_prints_exactly_one_url_line() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    assert!(parse_url_line(&server.url_line).is_some());
    assert_ne!(server.port, 0);
    let rest = server.stop();
    assert_eq!(rest, "", "nothing after the URL line on stdout");
}

#[test]
fn api_snapshot_with_the_token_is_the_snapshot() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    let response = server.get("/api/snapshot", &[("X-Repoview-Token", &server.token)]);
    assert_eq!(response.status, 200);
    assert!(response.content_type().starts_with("application/json"));
    let snapshot: Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        snapshot["project"]["root"],
        json!(canonical(project.path()))
    );
    assert_eq!(snapshot["sources"].as_array().unwrap().len(), 5);
}

#[test]
fn api_snapshot_accepts_the_token_query_parameter() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    let response = server.get(&format!("/api/snapshot?token={}", server.token), &[]);
    assert_eq!(response.status, 200);
}

#[test]
fn api_snapshot_without_or_with_a_wrong_token_is_403() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    assert_eq!(server.get("/api/snapshot", &[]).status, 403);
    let wrong = "0".repeat(64);
    assert_eq!(
        server
            .get("/api/snapshot", &[("X-Repoview-Token", &wrong)])
            .status,
        403
    );
    assert_eq!(
        server
            .get(&format!("/api/snapshot?token={wrong}"), &[])
            .status,
        403
    );
}

#[test]
fn unknown_api_path_with_the_token_is_404() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    let response = server.get("/api/nope", &[("X-Repoview-Token", &server.token)]);
    assert_eq!(response.status, 404);
}

#[test]
fn foreign_host_is_403_for_api_and_static() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    let evil = format!("evil.example:{}", server.port);
    let token = server.token.clone();
    for path in ["/api/snapshot", "/", "/board", "/assets/app.js"] {
        let response = server.get_with_host(path, &evil, &[("X-Repoview-Token", &token)]);
        assert_eq!(response.status, 403, "{path}");
    }
    let localhost = format!("localhost:{}", server.port);
    assert_eq!(server.get_with_host("/", &localhost, &[]).status, 200);
    let wrong_port = format!("127.0.0.1:{}", server.port.wrapping_add(1));
    assert_eq!(server.get_with_host("/", &wrong_port, &[]).status, 403);
}

#[test]
fn root_is_html_and_unknown_paths_fall_back_to_it() {
    let project = tempfile::tempdir().unwrap();
    let server = open_server(project.path());
    let root = server.get("/", &[]);
    assert_eq!(root.status, 200);
    assert!(root.content_type().starts_with("text/html"));
    let board = server.get("/board", &[]);
    assert_eq!(board.status, 200);
    assert_eq!(board.body, root.body);
}

// Acceptance 6: no flag and no environment variable widens the bind address.

#[test]
fn host_and_bind_flags_are_unknown_arguments() {
    for flag in ["--host", "--bind"] {
        let output = repoview()
            .args(["open", "--no-browser", flag, "0.0.0.0"])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{flag}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unexpected argument"), "{flag}: {stderr}");
        assert!(stderr.contains(flag), "{flag}: {stderr}");
    }
}

#[test]
fn bind_environment_variables_do_not_widen_the_address() {
    let project = tempfile::tempdir().unwrap();
    let mut command = repoview();
    command.args([
        "open",
        "--no-browser",
        "--port",
        "0",
        "--root",
        project.path().to_str().unwrap(),
    ]);
    for name in [
        "HOST",
        "BIND",
        "REPOVIEW_HOST",
        "REPOVIEW_BIND",
        "REPOVIEW_ADDR",
    ] {
        command.env(name, "0.0.0.0");
    }
    let server = Server::start_with(&mut command);
    assert!(server.url_line.starts_with("http://127.0.0.1:"));
    assert_eq!(server.get("/", &[]).status, 200);
}

// Acceptance 9: bare `repoview` is `repoview open`.

#[test]
fn bare_invocation_behaves_as_open() {
    let project = tempfile::tempdir().unwrap();
    let server = Server::start(&[
        "--no-browser",
        "--port",
        "0",
        "--root",
        project.path().to_str().unwrap(),
    ]);
    let response = server.get("/api/snapshot", &[("X-Repoview-Token", &server.token)]);
    assert_eq!(response.status, 200);
    assert_eq!(server.stop(), "");
}

// Acceptance 10: doctor.

#[test]
fn doctor_prints_one_line_per_source() {
    let repo = git_repo_with_commit();
    std::fs::write(repo.path().join("README.md"), "# x").unwrap();
    std::fs::create_dir(repo.path().join(".engineering")).unwrap();
    std::fs::write(repo.path().join(".engineering/project.yaml"), "x").unwrap();
    let path = bin_dir(&["git"]);
    let output = repoview()
        .args(["doctor", "--root"])
        .arg(repo.path())
        .env("PATH", path.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(lines.len(), 5, "{stdout}");
    let heads: Vec<(&str, &str)> = lines.iter().map(|l| (l[0], l[1])).collect();
    assert_eq!(
        heads,
        vec![
            ("vcs", "Present"),
            ("plan", "ToolMissing"),
            ("spec", "Absent"),
            ("quality", "Absent"),
            ("docs", "Present"),
        ],
        "{stdout}"
    );
    let git_path = path.path().join("git");
    assert!(
        stdout
            .lines()
            .next()
            .unwrap()
            .contains(git_path.to_str().unwrap()),
        "{stdout}"
    );
    assert!(
        stdout.lines().next().unwrap().contains("git version"),
        "{stdout}"
    );
    assert!(stdout.lines().nth(1).unwrap().contains("aep"), "{stdout}");
}

// Correction round 1.

/// Finding 4: `--root <file>` fails on every verb with a message naming the problem.
#[test]
fn root_pointing_at_a_file_fails_on_every_verb() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("README.md");
    std::fs::write(&file, "# x").unwrap();
    for verb in [
        &["doctor"][..],
        &["snapshot"],
        &["open", "--no-browser", "--port", "0"],
    ] {
        let output = repoview()
            .args(verb)
            .arg("--root")
            .arg(&file)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{verb:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not a directory"), "{verb:?}: {stderr}");
        assert!(output.stdout.is_empty(), "{verb:?}");
    }
}

/// A `/bin/sh` stub that appends its first argument to `record`, waited until executable.
fn recording_opener(dir: &Path, record: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let opener = dir.join("opener");
    std::fs::write(
        &opener,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&opener, std::fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match std::process::Command::new(&opener).arg("probe").output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            _ => break,
        }
    }
    std::fs::remove_file(record).ok();
    opener
}

fn wait_for(path: &Path) -> String {
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(5) {
        if let Ok(text) = std::fs::read_to_string(path)
            && text.ends_with('\n')
        {
            return text;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("{} never written", path.display());
}

/// Findings 5 and 7: the opener gets a `file://` URL of a private redirect page, never the
/// token; the page lives under `$XDG_CACHE_HOME/repoview` (0700) as a 0600 file holding the URL,
/// and is deleted when the server exits.
#[test]
fn browser_gets_a_private_redirect_file_not_the_token() {
    use std::os::unix::fs::PermissionsExt;
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let tools = tempfile::tempdir().unwrap();
    let record = tools.path().join("argv.txt");
    let opener = recording_opener(tools.path(), &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .env("BROWSER", &opener)
        .env("XDG_CACHE_HOME", cache.path());
    let server = Server::start_with(&mut command);
    let argument = wait_for(&record).trim().to_owned();
    assert!(!argument.contains(&server.token), "{argument}");
    let page = std::path::PathBuf::from(argument.strip_prefix("file://").expect("a file URL"));
    assert_eq!(page.parent().unwrap(), cache.path().join("repoview"));
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(page.parent().unwrap()), 0o700);
    assert_eq!(mode(&page), 0o600);
    let html = std::fs::read_to_string(&page).unwrap();
    assert!(html.contains(server.url_line.trim()), "{html}");
    assert!(!page.starts_with(project.path()));

    let status = std::process::Command::new("kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    let mut server = server;
    let started = std::time::Instant::now();
    while server.child.try_wait().unwrap().is_none() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "no exit on SIGTERM"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!page.exists(), "redirect page left behind");
}

/// Finding 5: a relative `$BROWSER` is never resolved against the working directory.
#[test]
fn relative_browser_variable_is_not_run_from_the_project() {
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let record = project.path().join("argv.txt");
    recording_opener(project.path(), &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .current_dir(project.path())
        .env("BROWSER", "./opener")
        .env("XDG_CACHE_HOME", cache.path());
    let server = Server::start_with(&mut command);
    std::thread::sleep(std::time::Duration::from_millis(500));
    drop(server);
    assert!(!record.exists(), "ran ./opener from the project directory");
}

// Correction round 2.

fn send(server: &Server, signal: &str) {
    let status = std::process::Command::new("kill")
        .args([&format!("-{signal}"), &server.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
}

/// Seconds until the server exits, or `None` past `limit`.
fn exit_time(server: &mut Server, limit: std::time::Duration) -> Option<std::time::Duration> {
    let started = std::time::Instant::now();
    while started.elapsed() < limit {
        if server.child.try_wait().unwrap().is_some() {
            return Some(started.elapsed());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    None
}

/// Finding 1: one SIGTERM with a client holding a half-sent request ends the server within the
/// 2 s shutdown bound (plus slack), not never.
#[test]
fn one_signal_bounds_shutdown_to_two_seconds() {
    use std::io::Write;
    let project = tempfile::tempdir().unwrap();
    let mut server = open_server(project.path());
    let mut client = std::net::TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    write!(
        client,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        server.port
    )
    .unwrap();
    client.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    send(&server, "TERM");
    let took = exit_time(&mut server, std::time::Duration::from_secs(4));
    drop(client);
    assert!(took.is_some(), "no exit within 4 s of one SIGTERM");
}

/// A server with the recording opener and a private cache; returns the server, the page path.
fn open_with_page(project: &Path, cache: &Path, tools: &Path) -> (Server, std::path::PathBuf) {
    let record = tools.join("argv.txt");
    let opener = recording_opener(tools, &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project)
        .env("BROWSER", &opener)
        .env("XDG_CACHE_HOME", cache);
    let server = Server::start_with(&mut command);
    let argument = wait_for(&record).trim().to_owned();
    let page = std::path::PathBuf::from(argument.strip_prefix("file://").unwrap());
    (server, page)
}

/// Finding 4: SIGHUP, SIGQUIT and SIGINT each end the server and delete the redirect page.
#[test]
fn hup_quit_and_int_each_delete_the_redirect_page() {
    for signal in ["HUP", "QUIT", "INT"] {
        let project = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let tools = tempfile::tempdir().unwrap();
        let (mut server, page) = open_with_page(project.path(), cache.path(), tools.path());
        assert!(page.exists(), "{signal}");
        send(&server, signal);
        assert!(
            exit_time(&mut server, std::time::Duration::from_secs(4)).is_some(),
            "no exit on SIG{signal}"
        );
        assert!(!page.exists(), "page left behind after SIG{signal}");
    }
}

/// Finding 5: a cache directory that resolves into the project through a symlink is refused
/// before anything is created, also when the symlink's target does not exist yet.
#[test]
fn cache_symlinked_into_the_project_is_refused_before_creating_anything() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(project.path().join("later"), home.path().join(".cache")).unwrap();
    let tools = tempfile::tempdir().unwrap();
    let record = tools.path().join("argv.txt");
    let opener = recording_opener(tools.path(), &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .env("BROWSER", &opener)
        .env("HOME", home.path())
        .env_remove("XDG_CACHE_HOME");
    let server = Server::start_with(&mut command);
    std::thread::sleep(std::time::Duration::from_millis(500));
    drop(server);
    let entries: Vec<_> = std::fs::read_dir(project.path()).unwrap().collect();
    assert!(entries.is_empty(), "wrote into the project: {entries:?}");
    assert!(
        !record.exists(),
        "opener ran with a page inside the project"
    );
}

/// Finding 7: a private `$XDG_RUNTIME_DIR` (absolute, ours, 0700) is preferred over the cache.
#[test]
fn private_runtime_dir_is_preferred_for_the_redirect_page() {
    use std::os::unix::fs::PermissionsExt;
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let runtime = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    std::fs::set_permissions(runtime.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tools = tempfile::tempdir().unwrap();
    let record = tools.path().join("argv.txt");
    let opener = recording_opener(tools.path(), &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .env("BROWSER", &opener)
        .env("XDG_CACHE_HOME", cache.path())
        .env("XDG_RUNTIME_DIR", runtime.path());
    let server = Server::start_with(&mut command);
    let argument = wait_for(&record).trim().to_owned();
    let page = std::path::PathBuf::from(argument.strip_prefix("file://").unwrap());
    assert_eq!(page.parent().unwrap(), runtime.path().join("repoview"));
    send(&server, "TERM");
    let mut server = server;
    assert!(exit_time(&mut server, std::time::Duration::from_secs(4)).is_some());
    assert!(!page.exists());
}

/// Finding 7: a `$XDG_RUNTIME_DIR` that is not 0700 is not used; the cache rule applies.
#[test]
fn runtime_dir_that_is_not_private_falls_back_to_the_cache() {
    use std::os::unix::fs::PermissionsExt;
    let project = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let runtime = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    std::fs::set_permissions(runtime.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let tools = tempfile::tempdir().unwrap();
    let record = tools.path().join("argv.txt");
    let opener = recording_opener(tools.path(), &record);
    let mut command = repoview();
    command
        .args(["open", "--port", "0", "--root"])
        .arg(project.path())
        .env("BROWSER", &opener)
        .env("XDG_CACHE_HOME", cache.path())
        .env("XDG_RUNTIME_DIR", runtime.path());
    let server = Server::start_with(&mut command);
    let argument = wait_for(&record).trim().to_owned();
    let page = std::path::PathBuf::from(argument.strip_prefix("file://").unwrap());
    assert_eq!(page.parent().unwrap(), cache.path().join("repoview"));
    assert!(!runtime.path().join("repoview").exists());
    send(&server, "TERM");
    let mut server = server;
    assert!(exit_time(&mut server, std::time::Duration::from_secs(4)).is_some());
}
