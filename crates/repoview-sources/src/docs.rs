//! `docs`: the top-level documents a project carries. No tool.

use serde_json::json;

use crate::{Availability, Detection, Env, Section, Source, SourceKind, blank};

pub(crate) struct Docs;

const LOCATION: &str = ".";
const FILES: [&str; 4] = ["README.md", "AGENTS.md", "STATUS.md", "CHANGELOG.md"];

fn present(env: &Env) -> Vec<&'static str> {
    FILES
        .into_iter()
        .filter(|name| env.root.join(name).is_file())
        .collect()
}

impl Source for Docs {
    fn id(&self) -> &'static str {
        "docs"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Documents
    }

    fn tool(&self) -> Option<&'static str> {
        None
    }

    fn detects(&self) -> &'static str {
        "README.md, AGENTS.md, STATUS.md or CHANGELOG.md"
    }

    fn detect(&self, env: &Env) -> Detection {
        if present(env).is_empty() {
            Detection::Absent
        } else {
            Detection::Detected
        }
    }

    fn read(&self, env: &Env) -> Section {
        let files = present(env);
        if files.is_empty() {
            return blank(self, LOCATION, Availability::Absent);
        }
        let mut section = blank(self, LOCATION, Availability::Present);
        section.summary = json!({ "files": files });
        section
    }
}
