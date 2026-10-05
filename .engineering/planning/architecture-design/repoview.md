---
format: aep.planning-md/3
id: architecture-design:repoview
kind: architecture-design
status: draft
title: 'repoview: one Rust binary, embedded Vue SPA, reading through the owning CLIs'
summary: Local 127.0.0.1 server with a run token; every fact read from git, aep, ess and codegate JSON output with its producer recorded.
relations:
- designs: vision:repoview
revision: 3
---
## Decision

repoview is one Rust binary with a Vue 3 single-page application embedded in it. Run in a project
directory, it finds the project root, starts an HTTP server on `127.0.0.1`, and opens the browser at
a URL that carries a per-run token. The server answers a JSON API; the SPA renders it.

Every fact comes from the tool that owns it, through that tool's own machine-readable output. repoview
does not parse an AEP store, an ESS document or a Git object itself. It runs `git`, `aep`, `ess` and
`codegate`, reads their JSON, and records which binary and version answered.

The first release is read-only. It writes nothing into the project directory.

## Context

The facts already have owners, and each owner already has a JSON surface (checked 2026-10-05 with
aep 0.68.0, ess 0.52.0):

| source | detected by | read with |
|---|---|---|
| Git | `git rev-parse --show-toplevel` | `git status --porcelain=v2 --branch`, `git log --format=…`, `git worktree list --porcelain`, `git tag`, `git remote -v` |
| AEP plan | `.engineering/project.yaml` | `aep plan artifact list/board/graph/validate/show/history/explain/blocked/waves --format json` |
| ESS | every `ess-inputs.yaml`, else `system.yaml` with `format: ess/…` | `ess specify validate --path <root> --format json`, `ess specify compile --path <root> --format json`, `ess specify graph --path <root> --format json` |
| Codegate | a `codegate` binary on `PATH` and a language it supports | `codegate --root . --language <l> --format json assess --gate all` |
| Documents | `README.md`, `AGENTS.md`, `STATUS.md`, `CHANGELOG.md`, `Taskfile.yml`, `Cargo.toml`, `package.json` | read as files |

Observed shapes:

- `aep plan artifact show <id> --format json` carries `id`, `kind`, `status`, `title`, `summary`,
  `relations`, `refs`, `scope`, `revision`, `findings`, `outcomes` and the markdown `body`.
- `aep plan artifact board --format json` is an array of status columns, each with its
  `artifacts`. The board's help says it exists so consumers stop hard-coding status blurbs; repoview
  takes columns and descriptions from it and never from a table of its own.
- `ess specify compile --format json` has top-level `system`, `version`, `naming`, `domains`,
  `types`, `conversions`, `entities`, `commands`, `events`, `errors`, `views`, `actors`,
  `bindings`, `components`, `workloads`.
- `ess specify graph --format json` has `system`, `version`, `groups`, `nodes`.
- The `codegate` on `PATH` today is the Go `fluxplane/codegate` (`~/go/bin/codegate`). Its
  `assess` JSON has `rating` (for example `B-`), `score_max`, `summary`, `scores` (`overall`,
  `boundary`, `test_boundary`, `coupling`, `side_effect`, `coverage`, `maintainability`,
  `pressure`), `validation`, `finding_counts`, `top_findings`. It supports `go` and `markdown`
  only; `--language rust` answers `language "rust" is not wired`.
- `beyond10x/codegate` (Rust, 0.3.0) has `evaluate --facts --policy` only. Its vision targets
  scoring with Go/Rust/Java parity against `fluxplane/codegate`
  (`codegate/.engineering/planning/vision/language-neutral-code-quality.md`).

`aep plan serve` already serves an AEP board with moves on `127.0.0.1` with a run token. repoview is
the cross-source view; it links to `aep plan serve` for moves until editing lands here.

## Why the CLIs and not their crates

Linking `aep` or `ess` crates would pin repoview to one release of each and to their internal APIs,
and a repository pinned to another protocol or format version would be read by the wrong code. The
installed CLIs are the authority their own skills name ("the CLI is the authority"). The cost is a
process per call and a dependency on the user's `PATH`; both are visible in the UI as the producer
line. A tool that is missing, too old or fails is a state the UI shows, with the diagnostic, and
never an empty panel.

## Components

```
repoview/
  Cargo.toml                 workspace
  crates/repoview/           the binary: clap derive CLI, axum server, embedded SPA
  crates/repoview-sources/   one module per source: vcs, plan, spec, quality, docs
  generated/                 Rust wire types generated from ess/
  web/                       Vue 3 + Vite + TypeScript SPA; built into crates/repoview at release
  ess/                       the read model specification
  Taskfile.yml               task check, task build, task dev
```

### CLI

clap derive. Verbs:

| command | does |
|---|---|
| `repoview` | same as `repoview open` |
| `repoview open [--port N] [--no-browser]` | discover the project from `PWD`, serve, open the browser |
| `repoview snapshot [--format json]` | print the whole read model and exit; for agents, tests and CI |
| `repoview doctor` | list each source: detected or not, tool path and version, last error |

`--root <dir>` on every verb overrides discovery. Discovery walks up from `PWD` to the Git top level;
outside Git it uses `PWD`.

### Sources

A source is a Rust type with two calls: `detect(root) -> Detection` and `read(root) -> Section`.
`Section` carries the owning tool's JSON unchanged, the producer (binary path and `--version`
output), the command line that produced it, the duration and an availability of `Present`, `Absent`,
`ToolMissing` or `Failed` (with the tool's stderr). Every subprocess runs with a timeout, with
`cwd` at the project root, and with no shell.

Sources run concurrently on first request and are cached in memory per process. Codegate assess is
the slow one (seconds on a repository of codegate's size); the snapshot returns without it and the
panel fills in when it finishes.

### Server

axum on tokio, bound to `127.0.0.1` with no flag to widen it. Every `/api/*` request must carry the
run token from the opened URL (query on first load, then the `X-Repoview-Token` header from the SPA);
static assets carry no project data and need none. The server also refuses a
`Host` header other than `127.0.0.1:<port>` or `localhost:<port>`, which stops DNS rebinding from
reading the project through another site. Neither is authentication; both are there now so the
later write endpoints do not inherit an open read surface.

API (all `GET`, all JSON):

| path | answers |
|---|---|
| `/api/snapshot` | project, every source with availability and producer |
| `/api/vcs` | branch, upstream ahead/behind, dirty files, recent commits, tags, remotes, worktrees |
| `/api/plan/board`, `/api/plan/artifacts`, `/api/plan/graph`, `/api/plan/validate` | AEP output as returned |
| `/api/plan/artifacts/{id}` (+ `/history`, `/explain`) | one artifact |
| `/api/spec/roots` | every ESS root with its validate result |
| `/api/spec/roots/{root}/ir`, `/graph` | compiled IR and interaction graph |
| `/api/quality` | codegate assessments per language, or why there is none |
| `/api/docs/{name}` | a top-level document's markdown |

### Frontend

Vue 3, Vite, TypeScript, vue-router; fetch composables, no state library until one is needed. Pages:

| page | shows |
|---|---|
| Overview | project name, Git line, one card per source with its availability and producer, Codegate rating |
| Plan · Board | columns and descriptions from `board --format json`; filter by kind, tag, text |
| Plan · Tree | vision → designs → epics → stories → tasks from `relations` (`serves`, `designs`, `decomposes`, `implements`) |
| Plan · Artifact | rendered body, relations as links, scope, history, explain, findings |
| Specs | per root: domains, entities with fields and relations, lifecycle as a state diagram, commands, events, views, components, interaction graph |
| Quality | rating, scores, finding counts, top findings, producer |
| Repository | status, commits, tags, worktrees; README, AGENTS.md, STATUS.md, CHANGELOG rendered |

Markdown from the repository renders through `markdown-it` and is sanitised with DOMPurify before
it reaches the DOM: a repository is input, and the page holds the run token. Diagrams (lifecycles,
the ESS graph, the plan graph) render with Mermaid, which `ess specify graph` already emits as a
format.

Types the SPA reads are generated, not hand-written: the read model is specified in `ess/`, ESS
projects it to JSON Schema or OpenAPI, and the TypeScript types are generated from that projection.
`task check` fails on drift. The owning tools' payloads are typed as opaque JSON in the read model
and given TypeScript types per format in `web/`.

### Build and distribution

The SPA is part of the binary. `task build` runs `pnpm build` in `web/` and then
`cargo build --release`; `rust-embed` compiles `web/dist` into `crates/repoview`, so the binary
serves the SPA from memory and a release is one file with nothing beside it. A release build with
no `web/dist` fails to compile rather than producing a binary that serves an empty page. `task dev`
runs Vite with a proxy to `repoview open --no-browser --port 7480`. The repository's running code
is Rust; `web/` is browser code and its Node toolchain builds it, nothing else. A build helper is a
Rust binary, never a Node script.

GitHub Actions produce the downloadable binary:

| workflow | trigger | produces |
|---|---|---|
| `ci.yml` | pull request, push to `main` | `task check`; on `main`, `repoview-<sha>-x86_64-unknown-linux-gnu.tar.gz` as a workflow artifact |
| `release-build.yml` | tag `0.*` | `repoview-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, plus `SHA256SUMS`, as workflow artifacts |
| `shared-gates.yml` | pull request, push, tag | the common Gates checks (`beyond10x/gates` reusable workflow) |

Each build smoke-tests the binary it packages: `repoview --version` equals the tag, and
`repoview snapshot --format json --root .` on the checkout reports Vcs `Present`, and a request to
`/` of a started server returns the embedded `index.html`. The workflows have read-only
permissions. The GitHub Release and its assets are published by `b10x-bot[bot]` through
`b10x-gates api` from the tag run's artifacts after their checksums verify, as codegate does
(`codegate/.github/workflows/release-build.yml`). An install is `curl` of the asset, checksum,
`tar -x` into `~/.local/bin`.

## Codegate

repoview reads an assessment document with a `rating`, not a particular binary. Today only
`fluxplane/codegate assess` produces one, for Go and Markdown. When `beyond10x/codegate` ships its
scoring (its own vision), its format becomes the one repoview targets and the Go adapter is removed.
A language with no assessment shows "not assessed: <tool> does not support <language>", never a
score.

## Later: editing

Editing goes through the owning CLI, the same way reads do: a lifecycle move is `aep plan artifact
move`, an approval is `aep plan artifact evidence --kind approval` with the operator named. repoview
never writes a store file, a specification or a Git object directly. Write endpoints will be `POST`
behind the same token and Host check, plus a confirmation step in the SPA.

## Open questions

1. Is a `Project` and its `Source`s an ESS entity with a one-state lifecycle (the current draft in
   `ess/domains/project.yaml`) or an immutable struct, as codegate models its facts? Settled by the
   shell epic's first story.
2. Does the read model reference the owning tools' formats by name, or carry them as `Json`
   (ess/15)? Marked `UNMAPPED:` in the draft.
3. Multi-repository view: `aep plan workspace` answers across the repositories a workspace names;
   whether repoview's workspace page is built on it is open until that epic is drafted.

## Specification

`ess/system.yaml` (`format: ess/20`, the newest ess 0.52.0 implements) and
`ess/domains/project.yaml`. `ess specify validate --path ess` printed
`repoview v1 — 2 file(s), valid`.
