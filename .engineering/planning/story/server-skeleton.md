---
format: aep.planning-md/3
id: story:server-skeleton
kind: story
status: implemented
title: 'Rust binary: CLI, token-guarded local server, snapshot of detected sources, embedded SPA'
summary: repoview open/snapshot/doctor over five detected sources; 127.0.0.1 with token and Host check; rust-embed of web/dist.
relations:
- decomposes: epic:shell
- serves: vision:repoview
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/repoview-sources/
- confidence: cited
  path: crates/repoview/
- confidence: cited
  path: rust-toolchain.toml
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T12:37:43Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-05T12:37:44Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-10-05T13:24:12Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":10,"verification":1}}}
---
## Story

As an engineer in a project directory, I run `repoview open --no-browser` and get a URL that serves
the web app and a token-guarded `/api/snapshot` describing the project's sources;
`repoview snapshot --format json` prints the same document and exits.

## Acceptance

1. `cargo test -p repoview -p repoview-sources` passes, and the red run before the implementation is
   in the implementor's report.
2. Discovery: run with no `--root` from a subdirectory of a Git repository, `project.root` is the
   repository's top level; run from a directory outside any Git repository, it is that directory;
   `--root <dir>` overrides both. One test per case.
3. In a fresh `git init` directory with one commit, `repoview snapshot --format json --root <dir>`
   exits 0 and prints a document whose `sources` has exactly five entries with `source_id`
   `vcs`, `plan`, `spec`, `quality`, `docs`; `vcs` is `Present` with `summary.branch` equal to
   `git branch --show-current` and `summary.head` equal to `git rev-parse HEAD`; `plan`, `spec`
   and `docs` are `Absent`; `quality` is `Absent` when no `codegate` is on `PATH`.
4. In a directory with `.engineering/project.yaml` and a `PATH` without `aep`, `plan` is
   `ToolMissing` with `tool` = `aep`. With a stub `aep` on `PATH` that prints `aep 9.9.9` for
   `--version`, `plan` is `Present` and `tool_version` is `aep 9.9.9`. With a stub that exits 3 and
   writes `boom` to stderr, `plan` is `Failed` and `diagnostic` contains `boom`.
5. `repoview open --no-browser --port 0 --root <dir>` prints exactly one line
   `http://127.0.0.1:<port>/?token=<64 hex chars>` and then, while it runs:
   - `GET /api/snapshot` with `X-Repoview-Token: <token>` → 200, `application/json`, the snapshot;
   - `GET /api/snapshot` with no token, or a wrong token → 403;
   - `GET /api/nope` with the valid token → 404;
   - any request (API or static) with `Host: evil.example:<port>` → 403;
   - `GET /` → 200 `text/html`; `GET /board` (unknown non-API path) → the same body as `GET /`.
6. Bind address: a test starts the server with every environment variable named `HOST`, `BIND`,
   `REPOVIEW_HOST`, `REPOVIEW_BIND`, `REPOVIEW_ADDR` set to `0.0.0.0`, and asserts the listener's
   `local_addr().ip()` is `127.0.0.1`; `repoview open --host 0.0.0.0` and `--bind 0.0.0.0` exit
   non-zero with clap's unknown-argument error.
7. Static assets: the router serves from an asset source behind a trait. A test injects an
   in-memory set holding `index.html` and `assets/app-abc.js`, and gets them back at `/` and
   `/assets/app-abc.js` with `text/html` and `text/javascript` content types. With no asset set
   (debug build, no `web/dist`), `GET /` returns a page containing `task build`.
8. `cargo build --release -p repoview` fails to compile when `web/dist/index.html` does not exist,
   with an error naming `web/dist` and `task build`.
9. Bare `repoview` (no subcommand) behaves as `repoview open`: a test runs `repoview --no-browser
   --port 0 --root <dir>` and gets the same single URL line as acceptance 5.
10. `repoview doctor --root <dir>` prints one line per source: id, availability, tool path and
   version or the reason.

## Wire contract (shared with `story:web-skeleton`)

**Snapshot.** `GET /api/snapshot` and `repoview snapshot --format json` return exactly this shape:

```json
{
  "repoview_version": "0.1.0",
  "project": { "root": "/abs/path/to/project", "name": "project" },
  "sources": [
    {
      "source_id": "vcs",
      "kind": "Vcs",
      "location": ".",
      "availability": "Present",
      "tool": "git",
      "tool_path": "/usr/bin/git",
      "tool_version": "git version 2.51.0",
      "diagnostic": null,
      "summary": { "branch": "main", "head": "5313961…", "dirty": 0 }
    }
  ]
}
```

- Fields follow `repoview.project.Source` in `ess/domains/project.yaml`. The ESS `project_root` field
  (the `via` of `Project.sources`) is not repeated on each source: `project.root` carries it once. `kind` is one of `Vcs`,
  `Planning`, `Specification`, `Quality`, `Documents`; `availability` one of `Present`, `Absent`,
  `ToolMissing`, `Failed`.
- `source_id` → `kind`: `vcs` → `Vcs`, `plan` → `Planning`, `spec` → `Specification`,
  `quality` → `Quality`, `docs` → `Documents`, in that order.
- `tool`, `tool_path`, `tool_version`, `diagnostic` are `null` when not applicable. `summary` is a
  JSON object, `{}` when there is nothing to say.

**Token transport.** The URL `repoview open` prints carries `?token=<64 hex chars>`. Every `/api/*`
request must carry the token as header `X-Repoview-Token` (the query parameter `token` is also
accepted); otherwise 403. Static assets (`/`, `/assets/*`, the SPA fallback) carry no project data
and need no token; every request, static or API, must carry `Host: 127.0.0.1:<port>` or
`localhost:<port>`, otherwise 403. This narrows the design's "every request carries the token" to
the API; `architecture-design:repoview` § Server records it.

## Detection

`vcs` = `git rev-parse --show-toplevel` succeeds; `plan` = `.engineering/project.yaml` exists (tool
`aep`, version from `aep --version`); `spec` = any `ess-inputs.yaml` or `system.yaml` under the
root outside Git-ignored paths (tool `ess`; `summary.roots` = their relative directories);
`quality` = `codegate` on `PATH` (tool `codegate`; Codegate is optional, so no binary is `Absent`,
not `ToolMissing`); `docs` = any of `README.md`, `AGENTS.md`, `STATUS.md`, `CHANGELOG.md` (no tool;
`summary.files` = those present). `Failed` when a tool exits non-zero; `diagnostic` = its stderr,
truncated to 4 KiB.

## Design notes

- Workspace: `Cargo.toml` (members `crates/*`), `rust-toolchain.toml` (`1.98.1`, rustfmt, clippy),
  `crates/repoview` (bin; clap derive; axum; tokio; rust-embed with `#[folder = "../../web/dist"]`
  and `allow_missing = true` for debug), `crates/repoview-sources` (lib: `Source` trait with
  `detect`/`read`, `Section`, the five sources).
- `web/dist` is produced by `story:web-skeleton` and by `task build`; this story does not depend on
  it. Debug builds compile without it (acceptance 7); release builds refuse (acceptance 8).
  `Taskfile.yml` is the coordinator's and exists on the integration branch before dispatch.
- Subprocesses: no shell, `current_dir` = project root, 10 s timeout each.
- Token: 32 random bytes from the OS RNG, hex-encoded; compared in constant time.
- Browser: `repoview open` without `--no-browser` launches `$BROWSER` if set, else `xdg-open` /
  `open`; failure to launch prints the URL and continues.

## Out of scope

Reading AEP, ESS, Codegate or Git beyond detection and the `summary` fields above (later epics);
wire types generated from `ess/` and the views marker in `ess/domains/project.yaml` (a later
story); file watching; any write.

