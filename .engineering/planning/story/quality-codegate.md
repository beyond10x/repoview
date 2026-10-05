---
format: aep.planning-md/3
id: story:quality-codegate
kind: story
status: implemented
title: Quality page on the beyond10x Codegate, not the Go binary
summary: Locate the Rust codegate by --version, name it, list its commands, state that 0.3.0 has no source assessment yet; never run the Go codegate.
relations:
- decomposes: epic:quality
- serves: vision:repoview
- depends_on: story:quality-page
scope:
- confidence: cited
  path: crates/repoview-sources/src/lib.rs
- confidence: cited
  path: crates/repoview-sources/src/quality.rs
- confidence: cited
  path: crates/repoview/src/api/quality.rs
- confidence: cited
  path: crates/repoview/tests/api_quality.rs
- confidence: cited
  path: web/src/api/quality.ts
- confidence: cited
  path: web/src/components/quality/
- confidence: cited
  path: web/src/pages/QualityPage.vue
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T14:27:38Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-05T14:27:38Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:16Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Story

As an engineer, the Quality page shows what the beyond10x Codegate (`github.com/beyond10x/codegate`,
Rust) can say about the project, names that binary and version, and says plainly what it cannot say
yet. The older Go `codegate` (`fluxplane/codegate`) is not used.

## Acceptance

1. Locating Codegate: repoview scans every absolute `PATH` entry, in order, for an executable named
   `codegate` whose `--version` prints `codegate <semver>` on stdout with exit 0, and takes the first.
   A `codegate` that fails `--version` (the Go one exits 1) is skipped and named in `skipped`. Rust
   tests use stubs for: Rust-style first, Go-style first then Rust-style, only Go-style, none.
2. `GET /api/quality` answers
   `{ "tool": "codegate", "tool_path", "tool_version", "skipped": [paths], "commands": [..], "assessment": null, "reason": "..." }`.
   `commands` is read from `codegate --help` (the subcommand list), never from a list in repoview.
   With codegate 0.3.0 (`evaluate` only), `assessment` is `null` and `reason` is
   `codegate 0.3.0 evaluates supplied dependency facts only; it has no source assessment yet`.
   No codegate found: 503 `{ "tool": "codegate", "exit": null, "stderr": "beyond10x codegate not found on PATH" }`.
3. Quality page: producer line (path, version), the commands it offers, the reason, and a link to
   `https://github.com/beyond10x/codegate`. No score, rating or finding is ever shown without an
   assessment (vitest). A 503 shows the stderr text.
4. The Overview `quality` card and the snapshot's `quality` source report the same binary and version
   (`repoview-sources/src/quality.rs` uses the same locator).
5. The background-run machinery from `story:quality-page` (single start, process-group tracking,
   shutdown barrier) is kept for the assessment command a later Codegate release adds; until then no
   assessment process starts.
6. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Notes

Operator, 2026-10-05: "focus on our codegate (new version) when providing these quality checks";
the Go functionality moves into beyond10x codegate later. Installed for development:
`~/.local/bin/codegate` 0.3.0 from the verified release asset. Supersedes the Go adapter in
`story:quality-page`, whose branch `impl/quality-page` is the base of this story.
