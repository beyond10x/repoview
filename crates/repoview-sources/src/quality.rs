//! `quality`: the beyond10x Codegate (`github.com/beyond10x/codegate`), located on `PATH`.
//!
//! A `codegate` is the beyond10x one when `codegate --version` prints `codegate <semver>` on
//! stdout and exits 0. Every absolute `PATH` entry is tried in order and the first such binary is
//! taken; every other `codegate` before it (the Go `fluxplane/codegate` exits 1 on `--version`) is
//! skipped and named. Codegate is optional, so none found is `Absent`, not `ToolMissing`. The
//! Quality API locates through [`Codegate::locate_with`], so the page and this source always name
//! the same binary and version.

use std::collections::HashSet;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{
    Availability, Detection, Env, Outcome, Section, Source, SourceKind, TIMEOUT, blank, run,
};

pub(crate) struct Quality;

const LOCATION: &str = ".";

/// The binary's name on `PATH`.
const NAME: &str = "codegate";

/// The beyond10x `codegate` found on `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codegate {
    pub path: PathBuf,
    /// The semver `codegate --version` printed, e.g. `0.3.0`.
    pub version: String,
}

/// What a search of `PATH` found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodegateSearch {
    /// The first beyond10x `codegate`, if any.
    pub found: Option<Codegate>,
    /// Every `codegate` tried before it whose `--version` did not identify it, in `PATH` order.
    pub skipped: Vec<PathBuf>,
}

impl Codegate {
    /// What a reader is told when no beyond10x `codegate` is on `PATH`.
    pub const NOT_FOUND: &str = "beyond10x codegate not found on PATH";

    /// Search `env`'s `PATH`, running each candidate's `--version` as [`run`] does.
    pub fn locate(env: &Env) -> CodegateSearch {
        Codegate::locate_with(env, |candidate| {
            run(env, candidate, &["--version"], TIMEOUT)
        })
    }

    /// Search `env`'s `PATH`, with `version` running one candidate's `codegate --version`.
    pub fn locate_with(env: &Env, mut version: impl FnMut(&Path) -> Outcome) -> CodegateSearch {
        let mut search = CodegateSearch::default();
        for candidate in candidates(env) {
            let found = match version(&candidate) {
                Outcome::Success { stdout } => Codegate::parse_version(&stdout),
                Outcome::Failure { .. } => None,
            };
            match found {
                Some(version) => {
                    search.found = Some(Codegate {
                        path: candidate,
                        version,
                    });
                    break;
                }
                None => search.skipped.push(candidate),
            }
        }
        search
    }

    /// The semver of `codegate <semver>`, the whole of `stdout` but surrounding whitespace.
    pub fn parse_version(stdout: &str) -> Option<String> {
        let version = stdout.trim().strip_prefix("codegate ")?;
        is_semver(version).then(|| version.to_owned())
    }
}

/// Every executable file called `codegate` in the absolute directories of `env`'s `PATH`, in
/// order, each file once: a file reached again (a directory listed twice, or under a second name
/// such as `/bin` -> `usr/bin`, or a symlink to it) is the binary already tried, so it is
/// identified by device and inode, not by path.
fn candidates(env: &Env) -> Vec<PathBuf> {
    let Some(path) = env.search_path() else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(NAME))
        .filter(|candidate| {
            candidate.metadata().is_ok_and(|meta| {
                meta.is_file()
                    && meta.permissions().mode() & 0o111 != 0
                    && seen.insert((meta.dev(), meta.ino()))
            })
        })
        .collect()
}

/// `MAJOR.MINOR.PATCH[-PRE][+BUILD]` as semver.org 2.0.0 defines it.
fn is_semver(text: &str) -> bool {
    let (rest, build) = match text.split_once('+') {
        Some((rest, build)) => (rest, Some(build)),
        None => (text, None),
    };
    let (core, pre) = match rest.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (rest, None),
    };
    let numeric = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    };
    let identifier = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    };
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| numeric(part))
        && pre.is_none_or(|pre| {
            pre.split('.').all(|part| {
                identifier(part) && (!part.bytes().all(|b| b.is_ascii_digit()) || numeric(part))
            })
        })
        && build.is_none_or(|build| build.split('.').all(identifier))
}

fn skipped_summary(skipped: &[PathBuf]) -> serde_json::Value {
    let skipped: Vec<_> = skipped
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    json!({ "skipped": skipped })
}

impl Source for Quality {
    fn id(&self) -> &'static str {
        "quality"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Quality
    }

    fn tool(&self) -> Option<&'static str> {
        Some(NAME)
    }

    fn detects(&self) -> &'static str {
        "a codegate on PATH whose --version prints `codegate <semver>`"
    }

    fn detect(&self, env: &Env) -> Detection {
        match Codegate::locate(env).found {
            Some(_) => Detection::Detected,
            None => Detection::Absent,
        }
    }

    fn read(&self, env: &Env) -> Section {
        let search = Codegate::locate(env);
        let mut section = match &search.found {
            Some(codegate) => {
                let mut section = blank(self, LOCATION, Availability::Present);
                section.tool_path = Some(codegate.path.to_string_lossy().into_owned());
                section.tool_version = Some(codegate.version.clone());
                section
            }
            None => {
                let mut section = blank(self, LOCATION, Availability::Absent);
                if !search.skipped.is_empty() {
                    let names: Vec<_> = search
                        .skipped
                        .iter()
                        .map(|path| path.to_string_lossy())
                        .collect();
                    section.diagnostic = Some(format!(
                        "{}; skipped: {}",
                        Codegate::NOT_FOUND,
                        names.join(", ")
                    ));
                }
                section
            }
        };
        section.summary = skipped_summary(&search.skipped);
        section
    }
}
