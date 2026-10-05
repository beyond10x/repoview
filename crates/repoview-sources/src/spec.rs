//! `spec`: ESS specification roots, detected by `ess-inputs.yaml` or `system.yaml` files outside
//! Git-ignored paths.

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{
    Availability, Detection, Env, Outcome, Section, Source, SourceKind, TIMEOUT, blank, run,
    with_tool_version,
};

pub(crate) struct Spec;

const LOCATION: &str = ".";
const MARKERS: [&str; 2] = ["ess-inputs.yaml", "system.yaml"];

impl Source for Spec {
    fn id(&self) -> &'static str {
        "spec"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Specification
    }

    fn tool(&self) -> Option<&'static str> {
        Some("ess")
    }

    fn detects(&self) -> &'static str {
        "ess-inputs.yaml or system.yaml outside Git-ignored paths"
    }

    fn detect(&self, env: &Env) -> Detection {
        if roots(env, LIMITS).roots.is_empty() {
            Detection::Absent
        } else {
            Detection::Detected
        }
    }

    fn read(&self, env: &Env) -> Section {
        let found = roots(env, LIMITS);
        let mut summary = json!({ "roots": found.roots });
        if found.truncated {
            summary["truncated"] = json!(true);
        }
        if found.roots.is_empty() {
            let mut section = blank(self, LOCATION, Availability::Absent);
            if found.truncated {
                section.summary = json!({ "truncated": true });
            }
            return section;
        }
        with_tool_version(self, env, LOCATION, |_| Ok(summary))
    }
}

/// Where the walk outside Git stops.
#[derive(Debug, Clone, Copy)]
struct Limits {
    /// The deepest directory checked for markers, counted from the root (depth 0).
    max_depth: usize,
    /// The most directory entries listed.
    max_entries: usize,
}

const LIMITS: Limits = Limits {
    max_depth: 6,
    max_entries: 20_000,
};

/// Directories the walk outside Git never enters.
const SKIPPED: [&str; 3] = [".git", "target", "node_modules"];

/// Specification roots found, and whether a limit cut the walk short.
#[derive(Debug, PartialEq, Eq)]
struct Found {
    /// Directories relative to the root (`.` for the root itself) holding a marker file.
    roots: Vec<String>,
    truncated: bool,
}

/// The specification roots the `spec` source detects, relative to the project root (`.` for the
/// root itself), in order. Runs no `ess`: `git ls-files` inside Git, a bounded walk outside it.
pub fn detected_roots(env: &Env) -> Vec<String> {
    roots(env, LIMITS).roots
}

fn roots(env: &Env, limits: Limits) -> Found {
    let Some(files) = git_files(env) else {
        return walk(&env.root, limits);
    };
    let roots: BTreeSet<String> = files
        .iter()
        .map(Path::new)
        .filter(|file| {
            file.file_name()
                .is_some_and(|name| MARKERS.iter().any(|marker| name == *marker))
        })
        .map(|file| relative_name(file.parent().unwrap_or(Path::new(""))))
        .collect();
    Found {
        roots: roots.into_iter().collect(),
        truncated: false,
    }
}

fn relative_name(dir: &Path) -> String {
    if dir.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        dir.to_string_lossy().into_owned()
    }
}

/// Tracked and untracked-but-not-ignored files under the root, or `None` outside Git.
fn git_files(env: &Env) -> Option<Vec<String>> {
    let git = env.find_tool("git")?;
    let args = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ];
    match run(env, &git, &args, TIMEOUT) {
        Outcome::Success { stdout } => Some(
            stdout
                .split('\0')
                .filter(|file| !file.is_empty())
                .map(str::to_owned)
                .collect(),
        ),
        Outcome::Failure { .. } => None,
    }
}

/// Breadth-first from `root`. Every queued directory is checked for the marker files directly,
/// which costs no budget; listing a directory to find its subdirectories spends one unit per
/// entry. So a large directory can use up the budget, but cannot hide a marker in a sibling
/// that was already queued. [`SKIPPED`] directories and symlinks are never entered.
fn walk(root: &Path, limits: Limits) -> Found {
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::from([(PathBuf::new(), 0)]);
    let mut budget = limits.max_entries;
    let mut truncated = false;
    let mut roots = BTreeSet::new();
    while let Some((relative, depth)) = queue.pop_front() {
        let dir = root.join(&relative);
        if MARKERS.iter().any(|marker| dir.join(marker).is_file()) {
            roots.insert(relative_name(&relative));
        }
        if budget == 0 {
            truncated = true;
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if budget == 0 {
                truncated = true;
                break;
            }
            budget -= 1;
            let name = entry.file_name();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if !is_dir || SKIPPED.iter().any(|skipped| name == *skipped) {
                continue;
            }
            if depth + 1 > limits.max_depth {
                truncated = true;
                continue;
            }
            queue.push_back((relative.join(&name), depth + 1));
        }
    }
    Found {
        roots: roots.into_iter().collect(),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::{Found, LIMITS, Limits, roots};
    use crate::Env;
    use std::fs;

    fn limits(max_entries: usize) -> Limits {
        Limits {
            max_depth: 6,
            max_entries,
        }
    }

    /// No `git` on `PATH`, so `roots` walks.
    fn walk_env(root: &std::path::Path, empty: &std::path::Path) -> Env {
        Env::with_path(root, empty)
    }

    #[test]
    fn roots_past_the_budget_report_truncation_and_keep_queued_markers() {
        let root = tempfile::tempdir().unwrap();
        let empty = tempfile::tempdir().unwrap();
        for dir in ["a", "b", "c/d"] {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        for index in 0..10 {
            fs::write(root.path().join(format!("a/f{index}")), "x").unwrap();
        }
        fs::write(root.path().join("b/system.yaml"), "x").unwrap();
        fs::write(root.path().join("c/d/system.yaml"), "x").unwrap();
        // Listing the root spends the whole budget of 3 on a, b and c: each is still checked for
        // markers, none is listed, so c/d is never queued and the ten files in a are never read.
        let found = roots(&walk_env(root.path(), empty.path()), limits(3));
        assert_eq!(
            found,
            Found {
                roots: vec!["b".to_owned()],
                truncated: true
            }
        );
    }

    #[test]
    fn roots_within_the_budget_are_not_truncated() {
        let root = tempfile::tempdir().unwrap();
        let empty = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("c/d")).unwrap();
        fs::write(root.path().join("c/d/system.yaml"), "x").unwrap();
        let found = roots(&walk_env(root.path(), empty.path()), limits(5));
        assert_eq!(
            found,
            Found {
                roots: vec!["c/d".to_owned()],
                truncated: false
            }
        );
    }

    #[test]
    fn production_limits_are_twenty_thousand_entries_and_depth_six() {
        assert_eq!(LIMITS.max_entries, 20_000);
        assert_eq!(LIMITS.max_depth, 6);
    }
}
