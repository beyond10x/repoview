---
format: aep.planning-md/3
id: epic:shell
kind: epic
status: draft
title: CLI, local server and Overview page
summary: repoview opens a token-guarded 127.0.0.1 view; snapshot and doctor answer the same read model headless.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- informed_by: executable-system-specification:read-model
revision: 3
---
## Outcome

`repoview` run in a project directory opens a browser tab on an Overview page that lists every
source with its availability and producer. `repoview snapshot --format json` prints the same read
model and exits. `repoview doctor` lists each source's detection, tool path and version.

## Scope

- The Cargo workspace, `crates/repoview` (clap derive CLI, axum server) and
  `crates/repoview-sources` with the `Source` trait and the `Section` record.
- Project discovery: Git top level from `PWD`, else `PWD`; `--root` overrides.
- Server on `127.0.0.1` only, run token, `Host` check, `/api/snapshot`.
- `web/` scaffold (Vue 3, Vite, TypeScript, vue-router), the Overview page, `rust-embed` of
  `web/dist`, `task build`, `task dev`, `task check`.
- The SPA data client's static mode (`./data/*.json`, hash routing, relative asset base), which
  `epic:static-export` builds on.
- Wire types generated from `ess/`; drift check in `task check` (a later story, after wave one).
- `Taskfile.yml` (`task check`, `task build`, `task dev`) is written by the wave coordinator before
  the stories are dispatched.

## Acceptance

- In a fresh Git repository with no other sources, Overview shows Vcs `Present` and the other four
  kinds `Absent`; `snapshot --format json` shows the same five availabilities.
- With `aep` removed from `PATH` in a repository that has `.engineering/project.yaml`, Planning is
  `ToolMissing` and names `aep`.
- Integration, checked by the wave coordinator after both stories merge: `task build`, then
  `repoview open --no-browser` in this repository; a browser at the printed URL shows the Overview
  page with `vcs` `Present` and its branch.
- An `/api/*` request without the run token, or any request with `Host: evil.example`, is refused
  with 403. Static assets carry no project data and need no token.

## Nouns

`repoview.project.Project` and `repoview.project.Source` in `ess/domains/project.yaml`
(`executable-system-specification:read-model`). The payload marker was settled before the first
story: `Source` carries `tool`, `tool_path` and `summary: Json` (2026-10-05). The views marker is
settled by the wire-types story.
