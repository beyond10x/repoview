//! Helpers shared by the process-level tests.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use tempfile::TempDir;

pub fn repoview() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_repoview"));
    command.env_remove("BROWSER");
    // Without this the redirect page would go to the real `$XDG_RUNTIME_DIR/repoview`.
    command.env_remove("XDG_RUNTIME_DIR");
    command
}

/// The real binary called `name` on the test process's `PATH`.
pub fn real_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{name} is on the test PATH"))
}

/// A directory to use as `PATH`, holding symlinks to the named real tools.
pub fn bin_dir(real: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in real {
        std::os::unix::fs::symlink(real_tool(name), dir.path().join(name)).unwrap();
    }
    dir
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=repoview test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

pub fn git_repo_with_commit() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    git(
        dir.path(),
        &["commit", "--quiet", "--allow-empty", "-m", "first"],
    );
    dir
}

pub fn canonical(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

/// A running `repoview` server process, killed on drop.
pub struct Server {
    pub child: Child,
    pub stdout: BufReader<ChildStdout>,
    pub url_line: String,
    pub port: u16,
    pub token: String,
}

impl Server {
    /// Start `repoview <args>` and read the URL line it prints.
    pub fn start(args: &[&str]) -> Server {
        Self::start_with(repoview().args(args))
    }

    pub fn start_with(command: &mut Command) -> Server {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut url_line = String::new();
        stdout.read_line(&mut url_line).unwrap();
        let (port, token) = parse_url_line(&url_line)
            .unwrap_or_else(|| panic!("not a repoview URL line: {url_line:?}"));
        Server {
            child,
            stdout,
            url_line,
            port,
            token,
        }
    }

    /// Stop the server and return what it printed after the URL line.
    pub fn stop(mut self) -> String {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        let mut rest = String::new();
        self.stdout.read_to_string(&mut rest).unwrap();
        rest
    }

    pub fn get(&self, path: &str, headers: &[(&str, &str)]) -> Response {
        let host = format!("127.0.0.1:{}", self.port);
        self.get_with_host(path, &host, headers)
    }

    pub fn get_with_host(&self, path: &str, host: &str, headers: &[(&str, &str)]) -> Response {
        http_get(self.port, path, host, headers)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `(port, token)` from `http://127.0.0.1:<port>/?token=<64 hex>\n`, or `None` if the line has
/// any other shape.
pub fn parse_url_line(line: &str) -> Option<(u16, String)> {
    let line = line.strip_suffix('\n')?;
    let rest = line.strip_prefix("http://127.0.0.1:")?;
    let (port, token) = rest.split_once("/?token=")?;
    let port: u16 = port.parse().ok()?;
    let valid = token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    valid.then(|| (port, token.to_owned()))
}

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn content_type(&self) -> &str {
        self.headers
            .get("content-type")
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// One HTTP/1.1 GET over a fresh connection, with `Connection: close`.
pub fn http_get(port: u16, path: &str, host: &str, headers: &[(&str, &str)]) -> Response {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header terminator");
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let mut body = raw[split + 4..].to_vec();
    let mut lines = head.split("\r\n");
    let status: u16 = lines
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers: HashMap<String, String> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    if headers.get("transfer-encoding").map(String::as_str) == Some("chunked") {
        body = dechunk(&body);
    }
    Response {
        status,
        headers,
        body,
    }
}

fn dechunk(mut raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let line_end = raw.windows(2).position(|w| w == b"\r\n").unwrap();
        let size =
            usize::from_str_radix(std::str::from_utf8(&raw[..line_end]).unwrap(), 16).unwrap();
        raw = &raw[line_end + 2..];
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&raw[..size]);
        raw = &raw[size + 2..];
    }
}
