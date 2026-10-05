//! Shared by `build.rs` and the crate's unit tests: a release build needs the built web app.

use std::path::Path;

/// `Err` with the message the build stops on when `profile` is `release` and `dist` has no
/// `index.html`.
pub fn check(profile: &str, dist: &Path) -> Result<(), String> {
    if profile != "release" || dist.join("index.html").is_file() {
        return Ok(());
    }
    Err(format!(
        "web/dist/index.html is missing ({}): a release build embeds the web app; run `task build`, \
         which builds web/dist and then the release binary",
        dist.display()
    ))
}

/// FNV-1a over every file under `dist` (relative path, then bytes), in path order; `absent` when
/// `dist` does not exist. Only used to make a changed web build change the compiler invocation.
pub fn digest(dist: &Path) -> String {
    let mut files = Vec::new();
    collect(dist, dist, &mut files);
    if files.is_empty() && !dist.is_dir() {
        return "absent".to_owned();
    }
    files.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for (relative, path) in files {
        feed(relative.as_bytes());
        feed(&[0]);
        feed(&std::fs::read(path).unwrap_or_default());
        feed(&[0]);
    }
    format!("{hash:016x}")
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push((relative.to_string_lossy().into_owned(), path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{check, digest};

    #[test]
    fn digest_changes_when_any_dist_file_changes_and_names_an_absent_dist() {
        let dist = tempfile::tempdir().unwrap();
        assert_eq!(digest(&dist.path().join("missing")), "absent");
        std::fs::create_dir(dist.path().join("assets")).unwrap();
        std::fs::write(dist.path().join("index.html"), "<!doctype html>").unwrap();
        std::fs::write(dist.path().join("assets/app.js"), "one").unwrap();
        let before = digest(dist.path());
        assert_eq!(before, digest(dist.path()));
        std::fs::write(dist.path().join("assets/app.js"), "two").unwrap();
        assert_ne!(before, digest(dist.path()));
    }

    #[test]
    fn release_without_index_html_names_web_dist_and_task_build() {
        let empty = tempfile::tempdir().unwrap();
        let message = check("release", empty.path()).unwrap_err();
        assert!(message.contains("web/dist"), "{message}");
        assert!(message.contains("task build"), "{message}");
    }

    #[test]
    fn release_with_index_html_builds() {
        let dist = tempfile::tempdir().unwrap();
        std::fs::write(dist.path().join("index.html"), "<!doctype html>").unwrap();
        assert_eq!(check("release", dist.path()), Ok(()));
    }

    #[test]
    fn debug_builds_without_web_dist() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(check("debug", &empty.path().join("missing")), Ok(()));
    }
}
