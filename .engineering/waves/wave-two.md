# Wave two

Coordinator: the repoview session, 2026-10-05. Skill: aep `implementing` 0.19.2, wave mode. The
operator's instruction to dispatch and implement covers this wave; no stage-1 stop. Commits covered:
one per unit, the merges into `wave/two`, the closing store commit, the merge into `main`.

## Selection

`aep plan artifact waves --kind story --status draft` (aep 0.68.0):

```
wave 1
  story:page-frame
wave 2
  story:plan-pages
  story:quality-page
  story:repository-page
  story:spec-pages
2 wave(s), 0 collision(s), 0 unassessed
```

Two stages on one integration branch: 2a is `story:page-frame` (the shared files every page needs),
2b the four page stories once 2a has merged. Scope typed and cited for all five. The plan-critic
panel was skipped: each decomposed epic gained one story (planning skill § 7, fewer than two).

## Pre-flight

| check | value |
|---|---|
| base | `main` at `c522f45` |
| free disk on `/` | 28G |
| build dirs from previous waves | wave one's server build dir renamed to the frame unit's (sequential reuse) |

## Units

| unit | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|
| story:page-frame | `impl/page-frame` | `~/.local/state/worktree/trees/b10x/repoview/impl-page-frame` | `~/.cache/b10x-target/repoview-page-frame` | `~/.cache/repoview-wave-two/page-frame/scratch` | dispatched |
| integration | `wave/two` | `~/.local/state/worktree/trees/b10x/repoview/wave-two` | reuses a finished unit's build dir | `~/.cache/repoview-wave-two/coordinator` | 2a |
| story:plan-pages | `impl/plan-pages` | `~/.local/state/worktree/trees/b10x/repoview/impl-plan-pages` | `~/.cache/b10x-target/repoview-plan-pages` | `~/.cache/repoview-wave-two/plan-pages/scratch` | dispatched |
| story:spec-pages | `impl/spec-pages` | `~/.local/state/worktree/trees/b10x/repoview/impl-spec-pages` | `~/.cache/b10x-target/repoview-spec-pages` | `~/.cache/repoview-wave-two/spec-pages/scratch` | dispatched |
| story:quality-page | `impl/quality-page` | `~/.local/state/worktree/trees/b10x/repoview/impl-quality-page` | `~/.cache/b10x-target/repoview-quality-page` | `~/.cache/repoview-wave-two/quality-page/scratch` | dispatched |
| story:repository-page | `impl/repository-page` | `~/.local/state/worktree/trees/b10x/repoview/impl-repository-page` | `~/.cache/b10x-target/repoview-repository-page` | `~/.cache/repoview-wave-two/repository-page/scratch` | dispatched |

Deviation from the two-stage plan: the four page units fork from the frame unit's commit `59771f6`
on `impl/page-frame`, not from `wave/two` after the frame merged, and were dispatched while the frame
adversary runs. Reason: the operator is waiting on visible pages. A frame correction lands on
`impl/page-frame`; each page branch merges into `wave/two` after the frame does. Each page build dir
was seeded with a copy of the frame's (1.1G) to skip cold builds.
| story:quality-codegate | `impl/quality-codegate` | `~/.local/state/worktree/trees/b10x/repoview/impl-quality-codegate` | `~/.cache/b10x-target/repoview-quality-codegate` | `~/.cache/repoview-wave-two/quality-codegate/scratch` | merged |

## Changes during the wave

- Operator, 2026-10-05: quality checks use the beyond10x Codegate (Rust), not the Go `fluxplane/codegate`
  that answered on `PATH`. `story:quality-page` (Go adapter) was not merged on its own;
  `story:quality-codegate` replaced the adapter on top of it and both merged together. Codegate 0.3.0
  was installed to `~/.local/bin/codegate` from the verified release asset.
- `story:repository-page`: the task list was removed (`task --list-all` evaluates `vars: sh:` with
  `dotenv:`); `story:taskfile-tasks` reads the Taskfile without executing it.
- `story:plan-pages`: the id pattern was amended to aep's grammar (`vision:O2`); all 4751 ids in 44
  stores match.
- Coordinator commits on `wave/two`: `e309fed` (project root as `Extension<Env>`), `8214b50` (JSON
  error bodies in `apiGet`).
- Follow-ups drafted: `story:shared-tool-runner`, `story:taskfile-tasks`.

## Close (2026-10-05)

| unit | adversary passes | findings per pass |
|---|---|---|
| story:page-frame | 2 | 2, then 1 note (routed to spec-pages) |
| story:plan-pages | 2 | 3, then 1 |
| story:spec-pages | 2 | 2, then 3 (1 a scope grant) |
| story:repository-page | 2 | 3, then 2 (1 pre-existing note) |
| story:quality-page | 2 | 3, then 1 |
| story:quality-codegate | 2 | 3, then 2 |

Each final correction was reviewed by the coordinator (adversary files unchanged, no ignored or
skipped cases, gates re-run). Gate on `wave/two` at `d8c59a5`, one exit per step: validate 0, ess
validate 0, fmt 0, clippy 0, cargo test 0 (188 passed, 30 binaries, `--list` 188), pnpm install 0,
pnpm check 0 (524 passed), task build 0.
