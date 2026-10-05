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

#[cfg(test)]
mod tests {
    use super::check;

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
