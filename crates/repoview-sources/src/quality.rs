//! `quality`: Codegate, detected by a `codegate` binary on `PATH`.
//!
//! Codegate is optional, so no binary is `Absent`, not `ToolMissing`. No version is probed: the
//! `codegate` on `PATH` today answers neither `--version` nor `version`.

use crate::{Availability, Detection, Env, Section, Source, SourceKind, blank};

pub(crate) struct Quality;

const LOCATION: &str = ".";

impl Source for Quality {
    fn id(&self) -> &'static str {
        "quality"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Quality
    }

    fn tool(&self) -> Option<&'static str> {
        Some("codegate")
    }

    fn detects(&self) -> &'static str {
        "codegate on PATH"
    }

    fn detect(&self, env: &Env) -> Detection {
        if env.find_tool("codegate").is_some() {
            Detection::Detected
        } else {
            Detection::Absent
        }
    }

    fn read(&self, env: &Env) -> Section {
        match env.find_tool("codegate") {
            Some(path) => {
                let mut section = blank(self, LOCATION, Availability::Present);
                section.tool_path = Some(path.to_string_lossy().into_owned());
                section
            }
            None => blank(self, LOCATION, Availability::Absent),
        }
    }
}
