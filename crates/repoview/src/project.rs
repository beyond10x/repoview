//! Project discovery and the snapshot document.

use std::io;
use std::path::{Path, PathBuf};

use repoview_sources::{Env, Outcome, Section, TIMEOUT, read_all, run};
use serde::{Deserialize, Serialize};

/// The snapshot `GET /api/snapshot` and `repoview snapshot --format json` return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub repoview_version: String,
    pub project: Project,
    pub sources: Vec<Section>,
}

/// `repoview.project.Project` on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub root: String,
    pub name: String,
}

/// The project root: `explicit` if given, else the Git top level above `cwd`, else `cwd`.
pub fn discover(cwd: &Path, explicit: Option<&Path>) -> io::Result<PathBuf> {
    if let Some(root) = explicit {
        let root = std::fs::canonicalize(cwd.join(root))?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!("{} is not a directory", root.display()),
            ));
        }
        return Ok(root);
    }
    let env = Env::new(cwd);
    if let Some(git) = env.find_tool("git")
        && let Outcome::Success { stdout } =
            run(&env, &git, &["rev-parse", "--show-toplevel"], TIMEOUT)
    {
        let top = stdout.trim_end_matches(['\n', '\r']);
        if !top.is_empty() {
            return std::fs::canonicalize(top);
        }
    }
    std::fs::canonicalize(cwd)
}

/// Read every source of the project at `env.root`.
pub fn snapshot(env: &Env) -> Snapshot {
    let name = env
        .root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| env.root.to_string_lossy().into_owned());
    Snapshot {
        repoview_version: env!("CARGO_PKG_VERSION").to_owned(),
        project: Project {
            root: env.root.to_string_lossy().into_owned(),
            name,
        },
        sources: read_all(env),
    }
}
