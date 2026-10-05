//! The sources repoview reads in one project directory.
//!
//! Each source answers two calls: [`Source::detect`] says whether its facts exist in the project
//! (or whether the tool needed to tell is missing), and [`Source::read`] returns the [`Section`]
//! the snapshot carries, with the producing tool and its version recorded.

mod docs;
mod plan;
mod process;
mod quality;
mod spec;
mod vcs;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use process::{DIAGNOSTIC_LIMIT, Outcome, TIMEOUT, find_tool, run, truncate_diagnostic};

/// `repoview.project.SourceKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    Vcs,
    Planning,
    Specification,
    Quality,
    Documents,
}

/// `repoview.project.Availability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    Present,
    Absent,
    ToolMissing,
    Failed,
}

impl Availability {
    pub fn as_str(self) -> &'static str {
        match self {
            Availability::Present => "Present",
            Availability::Absent => "Absent",
            Availability::ToolMissing => "ToolMissing",
            Availability::Failed => "Failed",
        }
    }
}

/// One entry of the snapshot's `sources`: the wire shape of `repoview.project.Source`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub source_id: String,
    pub kind: SourceKind,
    pub location: String,
    pub availability: Availability,
    pub tool: Option<String>,
    pub tool_path: Option<String>,
    pub tool_version: Option<String>,
    pub diagnostic: Option<String>,
    pub summary: Value,
}

/// What a source reads against: the project root and the `PATH` its tools are looked up on.
#[derive(Debug, Clone)]
pub struct Env {
    pub root: PathBuf,
    /// `None` uses the process `PATH`.
    pub path: Option<OsString>,
}

impl Env {
    /// An environment over `root` using the process `PATH`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Env {
            root: root.into(),
            path: None,
        }
    }

    /// An environment over `root` with an explicit `PATH`.
    pub fn with_path(root: impl Into<PathBuf>, path: impl Into<OsString>) -> Self {
        Env {
            root: root.into(),
            path: Some(path.into()),
        }
    }

    /// The `PATH` tools are searched on and children are started with.
    pub fn search_path(&self) -> Option<OsString> {
        self.path.clone().or_else(|| std::env::var_os("PATH"))
    }

    /// The first executable file called `name` on the search path.
    pub fn find_tool(&self, name: &str) -> Option<PathBuf> {
        process::find_tool(self.search_path().as_deref(), name)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Whether a source's facts exist in the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detection {
    Detected,
    Absent,
    /// The tool needed to tell is not on `PATH`.
    ToolMissing,
}

/// One source of facts in a project.
pub trait Source: Send + Sync {
    /// The stable `source_id`.
    fn id(&self) -> &'static str;
    fn kind(&self) -> SourceKind;
    /// The owning tool, if the source has one.
    fn tool(&self) -> Option<&'static str>;
    /// What detection looks for, in words, for `repoview doctor`.
    fn detects(&self) -> &'static str;
    fn detect(&self, env: &Env) -> Detection;
    fn read(&self, env: &Env) -> Section;
}

/// The five sources, in snapshot order: `vcs`, `plan`, `spec`, `quality`, `docs`.
pub fn all() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(vcs::Vcs),
        Box::new(plan::Plan),
        Box::new(spec::Spec),
        Box::new(quality::Quality),
        Box::new(docs::Docs),
    ]
}

/// Every source's section, in snapshot order.
pub fn read_all(env: &Env) -> Vec<Section> {
    all().iter().map(|source| source.read(env)).collect()
}

/// A section for `source` with every optional field `null` and an empty summary.
pub(crate) fn blank(source: &dyn Source, location: &str, availability: Availability) -> Section {
    Section {
        source_id: source.id().to_owned(),
        kind: source.kind(),
        location: location.to_owned(),
        availability,
        tool: source.tool().map(str::to_owned),
        tool_path: None,
        tool_version: None,
        diagnostic: None,
        summary: Value::Object(Default::default()),
    }
}

/// The section of a detected source whose tool answers `--version`: `ToolMissing` without the
/// tool, `Failed` with its stderr when it exits non-zero, else `Present` with `summary`.
pub(crate) fn with_tool_version(
    source: &dyn Source,
    env: &Env,
    location: &str,
    summary: impl FnOnce(&Path) -> Result<Value, String>,
) -> Section {
    let tool = source.tool().expect("a source with a tool");
    let Some(tool_path) = env.find_tool(tool) else {
        return tool_missing(source, location);
    };
    let mut section = blank(source, location, Availability::Present);
    section.tool_path = Some(tool_path.to_string_lossy().into_owned());
    match run(env, &tool_path, &["--version"], TIMEOUT) {
        Outcome::Success { stdout } => section.tool_version = Some(stdout.trim().to_owned()),
        Outcome::Failure { diagnostic } => {
            section.availability = Availability::Failed;
            section.diagnostic = Some(diagnostic);
            return section;
        }
    }
    match summary(&tool_path) {
        Ok(value) => section.summary = value,
        Err(diagnostic) => {
            section.availability = Availability::Failed;
            section.diagnostic = Some(diagnostic);
        }
    }
    section
}

pub(crate) fn tool_missing(source: &dyn Source, location: &str) -> Section {
    let tool = source.tool().expect("a source with a tool");
    let mut section = blank(source, location, Availability::ToolMissing);
    section.diagnostic = Some(format!("{tool} not found on PATH"));
    section
}
