# AGENTS.md — repoview

One CLI, `repoview`, that serves a read-only browser view of the project in its working directory:
Git, ESS specifications, the AEP plan, Codegate quality, top-level documents. Rust server, Vue 3
SPA embedded in the binary. Humans start at [`README.md`](README.md).

## Serves

- **O2 — decisions as data, with evidence.** Specifications, the plan and the quality rating are
  shown as the tools that own them report them, and every panel names the tool and version that
  answered.
- **O6 — self-improvement, built into all of it.** A project's Codegate rating and plan state sit
  on one page, so a change can be read before and after it lands.

## Map

| path | what |
|---|---|
| `.engineering/planning/` | the AEP store: `vision:repoview`, `architecture-design:repoview`, the epics |
| `ess/` | the read model specification (`format: ess/22`); `ess specify validate --path ess` |
| `crates/repoview/` | the binary: clap derive CLI, axum server, embedded SPA (planned) |
| `crates/repoview-sources/` | one module per source: vcs, plan, spec, quality, docs (planned) |
| `generated/` | Rust wire types generated from `ess/`; never hand-edited (planned) |
| `web/` | Vue 3 + Vite + TypeScript SPA (planned) |

## Rules

- **Running code is Rust**, with clap derive for the command line. `web/` is browser code; Node
  only builds and tests it. A build or check helper is a Rust binary, never a Node or Python script.
- **Read through the owning CLI.** Facts come from `git`, `aep`, `ess` and `codegate` output
  (`--format json` where the tool has it). Never parse an AEP store file, an ESS document or a Git
  object directly, and never keep a local copy of another tool's vocabulary (status names, column
  blurbs, format lists).
- **Missing is a state, not a zero.** A source is `Present`, `Absent`, `ToolMissing` or `Failed`
  with the tool's diagnostic. No panel renders an absent source as empty, and no score is shown for
  a language the assessing tool does not support.
- **Read-only.** repoview writes nothing inside the project directory. Later write paths go
  through the owning CLI (`aep plan artifact move`, `evidence`), never a file write.
- **Local only.** The server binds `127.0.0.1`, requires the run token and checks `Host`. Do not
  add a flag that widens the bind address.
- **Untrusted markdown.** Repository markdown is sanitised (DOMPurify) before it reaches the DOM.
- **One binary.** The SPA is embedded with `rust-embed`; a release build without `web/dist` must
  fail to compile.
- **ESS first.** A new noun gets its entry in `ess/` before a story is written around it. Wire
  types the SPA reads are generated from `ess/`; `task check` fails on drift.
- Use managed worktrees for changes; keep the primary checkout clean.
- Builds use `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/repoview`; check `df -h /` first and do
  not start a build under 10G free.
- Planning store mutations use `aep plan artifact` only (`aep:planning`). Scratch bodies for
  `--from` go in `.engineering/drafts/`, which is git-ignored.

## Gate

```console
task check
```

Planned contents: `aep plan artifact validate`, `ess specify validate --path ess`, generated drift,
`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and in `web/` typecheck, lint,
`vitest` and `vite build`.

## Publishing

Commits and pushes are `b10x-bot[bot]` through `b10x-gates bot`; every GitHub write goes through
`b10x-gates api` (workspace `AGENTS.md`, "GitHub writes are the bot's"). A release is an annotated
tag `0.x.y` on `main`; `release-build.yml` builds the archives and `SHA256SUMS` as workflow
artifacts with read-only permissions. Download that tag run's artifacts, verify the checksums,
then publish the GitHub Release and its assets as the bot. Report released only after the release
and its assets are verified.
