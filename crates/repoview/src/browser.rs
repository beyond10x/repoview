//! Opening the browser without putting the run token in a process's argv.
//!
//! The opener gets `file://` of a redirect page written into a private directory under the
//! user's cache directory; the page carries the token and is deleted when the server exits.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use repoview_sources::find_tool;

use crate::server::new_token;

/// A redirect page on disk, deleted on drop.
#[derive(Debug)]
pub struct RedirectPage {
    path: PathBuf,
}

impl RedirectPage {
    /// Write a mode-0600 page redirecting to `url` into [`redirect_dir`], which is created
    /// mode 0700. Refuses a directory that resolves, through symlinks, inside `project` or under
    /// a shared temporary directory; the check runs before anything is created and again after.
    pub fn create(url: &str, project: &Path) -> io::Result<RedirectPage> {
        let dir = redirect_dir()?;
        let project = fs::canonicalize(project)?;
        refuse_shared_or_project(&resolved(&dir), &project, &dir)?;
        private_dir(&dir)?;
        refuse_shared_or_project(&fs::canonicalize(&dir)?, &project, &dir)?;
        let path = dir.join(format!(
            "open-{}-{}.html",
            std::process::id(),
            &new_token()[..16]
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        let page = RedirectPage { path };
        let href = html_escape(url);
        write!(
            file,
            "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
             <meta http-equiv=\"refresh\" content=\"0;url={href}\">\
             <title>repoview</title></head>\n<body><a href=\"{href}\">{href}</a></body></html>\n"
        )?;
        Ok(page)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `file://` URL of the page.
    pub fn url(&self) -> String {
        file_url(&self.path)
    }
}

impl Drop for RedirectPage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn refuse_shared_or_project(resolved: &Path, project: &Path, dir: &Path) -> io::Result<()> {
    let shared = ["/tmp", "/var/tmp", "/dev/shm"]
        .iter()
        .any(|shared| resolved.starts_with(shared));
    if resolved.starts_with(project) || shared {
        return Err(io::Error::other(format!(
            "refusing {} (resolves to {}) for the browser redirect page",
            dir.display(),
            resolved.display()
        )));
    }
    Ok(())
}

/// `path` with its deepest existing ancestor canonicalized and the missing rest appended.
fn resolved(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut missing = Vec::new();
    loop {
        if let Ok(canonical) = fs::canonicalize(existing) {
            return missing
                .iter()
                .rev()
                .fold(canonical, |base, part| base.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_owned());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// `$XDG_RUNTIME_DIR/repoview` when `XDG_RUNTIME_DIR` is absolute, a real directory owned by
/// this user and mode 0700; else `$XDG_CACHE_HOME/repoview` when that is absolute; else
/// `$HOME/.cache/repoview`.
pub fn redirect_dir() -> io::Result<PathBuf> {
    if let Some(runtime) = private_runtime_dir() {
        return Ok(runtime.join("repoview"));
    }
    let cache = match std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => match std::env::var_os("HOME").map(PathBuf::from) {
            Some(home) if home.is_absolute() => home.join(".cache"),
            _ => return Err(io::Error::other("neither XDG_CACHE_HOME nor HOME is set")),
        },
    };
    Ok(cache.join("repoview"))
}

fn private_runtime_dir() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let dir = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    let meta = fs::symlink_metadata(&dir).ok()?;
    // SAFETY: getuid(2) has no preconditions and touches no memory.
    let uid = unsafe { libc::getuid() };
    let private =
        dir.is_absolute() && meta.is_dir() && meta.uid() == uid && meta.mode() & 0o777 == 0o700;
    private.then_some(dir)
}

/// Create `dir` mode 0700, or make an existing real directory 0700. A symlink is refused.
fn private_dir(dir: &Path) -> io::Result<()> {
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }
    match DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(dir)?;
            if !meta.is_dir() {
                return Err(io::Error::other(format!(
                    "{} is not a directory",
                    dir.display()
                )));
            }
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        }
        Err(error) => Err(error),
    }
}

/// The opener: `$BROWSER` (an absolute path, or a name looked up on `PATH`), else `xdg-open`
/// (`open` on macOS). Lookup uses only absolute `PATH` entries, so nothing runs from the
/// working directory.
pub fn opener() -> Option<PathBuf> {
    let path = std::env::var_os("PATH");
    let resolve = |name: &str| -> Option<PathBuf> {
        if name.contains('/') {
            let candidate = PathBuf::from(name);
            let executable = candidate.is_absolute()
                && candidate
                    .metadata()
                    .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
            executable.then_some(candidate)
        } else {
            find_tool(path.as_deref(), name)
        }
    };
    match std::env::var("BROWSER") {
        Ok(browser) if !browser.trim().is_empty() => resolve(browser.trim()),
        _ if cfg!(target_os = "macos") => resolve("open"),
        _ => resolve("xdg-open"),
    }
}

fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for byte in path.as_os_str().as_encoded_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                url.push(char::from(*byte))
            }
            other => url.push_str(&format!("%{other:02X}")),
        }
    }
    url
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::file_url;
    use std::path::Path;

    #[test]
    fn file_urls_percent_encode_everything_but_unreserved_and_slash() {
        assert_eq!(
            file_url(Path::new("/a b/c%d/é.html")),
            "file:///a%20b/c%25d/%C3%A9.html"
        );
    }
}
