//! Adversary cases, pass 2, for story:server-skeleton: shutdown and the browser redirect page.

mod common;

use std::io::Write;
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use common::{Server, repoview};

fn signal(server: &Server, name: &str) {
    let status = Command::new("kill")
        .args([&format!("-{name}"), &server.child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
}

/// Wait up to `limit` for the server to exit; `true` if it did.
fn exits_within(server: &mut Server, limit: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if server.child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// A `/bin/sh` opener that appends its first argument to `record`, waited until executable.
fn recording_opener(dir: &Path, record: &Path) -> PathBuf {
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
        match Command::new(&opener).arg("probe").output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => break,
        }
    }
    std::fs::remove_file(record).ok();
    opener
}

fn wait_for_line(path: &Path, limit: Duration) -> Option<String> {
    let started = Instant::now();
    while started.elapsed() < limit {
        if let Ok(text) = std::fs::read_to_string(path)
            && text.ends_with('\n')
        {
            return Some(text.trim().to_owned());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Ctrl-C stops `repoview open`. Before the redirect page, SIGINT's default action ended the
/// process; now a graceful shutdown waits for open connections, so one client that has sent half
/// a request (another local process, or a stalled browser socket) keeps the server alive, and a
/// second Ctrl-C is caught by the same handler and does nothing.
#[test]
fn sigint_exits_while_a_client_holds_a_half_sent_request() {
    let project = tempfile::tempdir().unwrap();
    let mut command = repoview();
    command
        .args(["open", "--no-browser", "--port", "0", "--root"])
        .arg(project.path());
    let mut server = Server::start_with(&mut command);
    let mut client = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    write!(
        client,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        server.port
    )
    .unwrap();
    client.flush().unwrap();
    std::thread::sleep(Duration::from_millis(300));

    signal(&server, "INT");
    std::thread::sleep(Duration::from_millis(300));
    signal(&server, "INT");
    let exited = exits_within(&mut server, Duration::from_secs(5));
    drop(client);
    assert!(
        exited,
        "repoview open still running 5 s after two SIGINTs while a client held a half-sent request"
    );
}

/// The redirect page holds the run token and is deleted when the server exits. Closing the
/// terminal `repoview open` runs in sends SIGHUP, which must not leave the page behind.
#[test]
fn sighup_does_not_leave_the_redirect_page_behind() {
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
    let mut server = Server::start_with(&mut command);
    let argument = wait_for_line(&record, Duration::from_secs(5)).expect("the opener ran");
    let page = PathBuf::from(argument.strip_prefix("file://").expect("a file URL"));
    assert!(page.exists(), "{}", page.display());

    signal(&server, "HUP");
    assert!(
        exits_within(&mut server, Duration::from_secs(5)),
        "no exit on SIGHUP"
    );
    let left = std::fs::read_to_string(&page).ok();
    assert!(
        left.is_none(),
        "redirect page with the run token left behind after SIGHUP: {}",
        page.display()
    );
}

/// "repoview writes nothing inside the project directory." `RedirectPage::create` refuses a
/// cache directory inside the project by comparing paths as written, so a `~/.cache` that is a
/// symlink into the project puts the page (and `repoview/`) inside the project.
#[test]
fn redirect_page_is_not_written_into_the_project_through_a_symlinked_cache() {
    let project = tempfile::tempdir().unwrap();
    let real_cache = project.path().join("cache");
    std::fs::create_dir(&real_cache).unwrap();
    let home = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(&real_cache, home.path().join(".cache")).unwrap();
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
    let argument = wait_for_line(&record, Duration::from_secs(3));
    let written = real_cache.join("repoview");
    let inside = written.exists();
    drop(server);
    assert!(
        !inside,
        "repoview wrote {} inside the project (opener got {argument:?})",
        written.display()
    );
}
