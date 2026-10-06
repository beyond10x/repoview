---
format: aep.planning-md/3
id: story:static-export
kind: story
status: implemented
title: repoview export writes a static copy of every page
summary: Embedded SPA in static mode plus one JSON per API answer from the real router; no token, no home paths; hash routing.
relations:
- decomposes: epic:static-export
- serves: vision:repoview
scope:
- confidence: cited
  path: crates/repoview/src/export.rs
- confidence: cited
  path: crates/repoview/src/lib.rs
- confidence: cited
  path: crates/repoview/src/main.rs
- confidence: cited
  path: crates/repoview/tests/export.rs
- confidence: cited
  path: web/src/api/client.ts
- confidence: cited
  path: web/src/router.ts
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T16:15:10Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-05T16:15:10Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-06T00:02:04Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
---
## Story

As an engineer, I run `repoview export --out site/` and get a directory any static file server can
serve: the same pages as `repoview open`, with every answer baked in as JSON and no server, no token.

## Acceptance

1. `repoview export --out <dir> [--root <dir>]` writes `<dir>/index.html` (the embedded one with
   `<meta name="repoview-mode" content="static">` instead of `server`), the embedded `assets/`, and
   one JSON file per API answer the pages read, at `<dir>/data/<api path>.json` (the static mapping in
   `web/src/api/client.ts`). The answers come from the real router (in-process requests with the run
   token), not from a second implementation.
2. Exported routes: `snapshot`; `plan/board`, `plan/artifacts`, `plan/graph`, `plan/validate`, and for
   every id in `plan/artifacts`: `plan/artifacts/{id}`, `/history`, `/explain`; `spec/roots` and for
   every root: `ir`, `graph`, `mermaid`; `quality`; `vcs`; `docs` and every present `docs/{name}`. A
   route that answers non-2xx is written as `<path>.error.json` with status and body, and listed in
   `<dir>/data/export.json` (`{ repoview_version, exported_at, routes: [{path, status}] }`).
3. No run token, no absolute path from the exporting user's home directory, and no `tool_path`
   values appear in any exported file (`tool_path` is replaced by the tool's file name). A Rust test
   greps the whole export for the token and for the home directory.
4. The SPA in static mode uses hash routing (`#/plan/...`), so deep links work on a static host; every
   page loads from `./data/` (vitest for the route mapping; a Rust test runs the export on a fixture
   project and checks every file the pages request exists).
5. `--out` must be empty or absent; an existing non-empty directory is refused (exit 2) unless
   `--force`, which only removes what a previous export wrote (it checks `data/export.json`).
6. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Notes

The public docs site (`docs` skill) embeds an export of this repository; that is the next story,
written by the coordinator after this one merges.
