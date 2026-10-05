//! `plan`: the AEP planning store, detected by `.engineering/project.yaml`.

use serde_json::json;

use crate::{Availability, Detection, Env, Section, Source, SourceKind, blank, with_tool_version};

pub(crate) struct Plan;

const LOCATION: &str = ".engineering";
const MARKER: &str = ".engineering/project.yaml";

impl Source for Plan {
    fn id(&self) -> &'static str {
        "plan"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Planning
    }

    fn tool(&self) -> Option<&'static str> {
        Some("aep")
    }

    fn detects(&self) -> &'static str {
        MARKER
    }

    fn detect(&self, env: &Env) -> Detection {
        if env.root.join(MARKER).is_file() {
            Detection::Detected
        } else {
            Detection::Absent
        }
    }

    fn read(&self, env: &Env) -> Section {
        match self.detect(env) {
            Detection::Detected => with_tool_version(self, env, LOCATION, |_| Ok(json!({}))),
            _ => blank(self, LOCATION, Availability::Absent),
        }
    }
}
