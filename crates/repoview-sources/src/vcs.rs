//! `vcs`: the Git work tree the project root lies in.

use serde_json::json;

use crate::{
    Availability, Detection, Env, Outcome, Section, Source, SourceKind, TIMEOUT, blank, run,
    tool_missing, with_tool_version,
};

pub(crate) struct Vcs;

const LOCATION: &str = ".";

impl Source for Vcs {
    fn id(&self) -> &'static str {
        "vcs"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Vcs
    }

    fn tool(&self) -> Option<&'static str> {
        Some("git")
    }

    fn detects(&self) -> &'static str {
        "a Git work tree (git rev-parse --show-toplevel)"
    }

    fn detect(&self, env: &Env) -> Detection {
        let Some(git) = env.find_tool("git") else {
            return Detection::ToolMissing;
        };
        match run(env, &git, &["rev-parse", "--show-toplevel"], TIMEOUT) {
            Outcome::Success { .. } => Detection::Detected,
            Outcome::Failure { .. } => Detection::Absent,
        }
    }

    fn read(&self, env: &Env) -> Section {
        match self.detect(env) {
            Detection::ToolMissing => return tool_missing(self, LOCATION),
            Detection::Absent => return blank(self, LOCATION, Availability::Absent),
            Detection::Detected => {}
        }
        with_tool_version(self, env, LOCATION, |git| {
            let branch = stdout(env, git, &["branch", "--show-current"])?;
            let head = match run(
                env,
                git,
                &["rev-parse", "--verify", "--quiet", "HEAD"],
                TIMEOUT,
            ) {
                Outcome::Success { stdout } => Some(stdout.trim().to_owned()),
                Outcome::Failure { .. } => None,
            };
            let status = stdout(env, git, &["--no-optional-locks", "status", "--porcelain"])?;
            let dirty = status.lines().filter(|line| !line.is_empty()).count();
            Ok(json!({ "branch": branch.trim(), "head": head, "dirty": dirty }))
        })
    }
}

fn stdout(env: &Env, git: &std::path::Path, args: &[&str]) -> Result<String, String> {
    match run(env, git, args, TIMEOUT) {
        Outcome::Success { stdout } => Ok(stdout),
        Outcome::Failure { diagnostic } => Err(diagnostic),
    }
}
