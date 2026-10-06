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
| `crates/repoview/` | the binary: clap derive CLI (`open`, `snapshot`, `doctor`, `export`), axum server, `api/` modules per page, embedded SPA (`build.rs` hashes `web/dist`) |
| `crates/repoview-sources/` | one module per source (vcs, plan, spec, quality, docs) and the one subprocess runner (`run_output`) |
| `web/` | Vue 3 + Vite + TypeScript SPA |
| `.github/workflows/` | `ci.yml` (gate; Linux build per `main` push), `release-build.yml` (tags: three targets, `SHA256SUMS`), `shared-gates.yml` |
| `.engineering/waves/` | one page per wave: units, gate, decisions |

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
- CI has no `TMPDIR`, so `tempfile::tempdir()` lands in `/tmp`, which `browser.rs` refuses for the
  token page. A test that hands repoview a cache or runtime directory creates it with
  `tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR"))`; run `TMPDIR=/tmp cargo test` before pushing
  such a test.
- Planning store mutations use `aep plan artifact` only (`aep:planning`). Scratch bodies for
  `--from` go in `.engineering/drafts/`, which is git-ignored.

## Gate

```console
task check
```

Steps (`Taskfile.yml`): `aep plan artifact validate`, `ess specify validate --path ess`,
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `pnpm --dir web install --frozen-lockfile`, `pnpm --dir web check`
(format, lint, typecheck, vitest, build). `task build` builds the web app, then the release binary
with it embedded. Wire types generated from `ess/` are not built yet (the views marker in
`ess/domains/project.yaml` is open).

## Publishing

Commits and pushes are `b10x-bot[bot]` through `b10x-gates bot`; every GitHub write goes through
`b10x-gates api` (workspace `AGENTS.md`, "GitHub writes are the bot's"). A release:

1. Bump `version` in `Cargo.toml` (workspace) and `web/package.json`; `task check` green on that commit.
2. Annotated tag `0.x.y` on `main`, pushed as the bot.
3. `release-build.yml` runs on the tag (contents: read): three targets, each smoke-tested, then the
   artifact `repoview-<tag>-release` with the three archives and `SHA256SUMS`.
4. Download that artifact (`gh run download <run> -n repoview-<tag>-release`, read-only), run
   `sha256sum -c SHA256SUMS`.
5. Create the GitHub Release and upload the four files through `b10x-gates api` as `b10x-bot[bot]`.
6. Report released only after the release, its assets and their checksums are verified.
