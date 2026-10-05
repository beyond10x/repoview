# Wave one

Coordinator: the repoview design session, 2026-10-05. Skill: aep `implementing` 0.19.2, wave mode.
Approval: the operator asked for the wave to be dispatched and implemented in the same session;
no stage-1 stop. Commits this approval covers: one per unit, the merges into `wave/one`, the
closing store commit, the merge of `wave/one` into `main`.

## Selection

`aep plan artifact waves --kind story --status draft` (aep 0.68.0), before the moves:

```
wave 1
  story:server-skeleton
  story:web-skeleton
1 wave(s), 0 collision(s), 0 unassessed
```

Scope is typed and cited for both (`aep plan artifact scope`). Both serve `vision:repoview` and
decompose `epic:shell`. Left out: every other epic; each depends on `epic:shell`.

Plan-critic panel: two rounds, records `review-result:shell-{acceptance,design,scope,parallel-safety}-round-{1,2}`.
Round 1: 4 × needs-revision, 12 findings. Round 2: acceptance and parallel-safety approve; design
and scope 1 finding each. All 14 fixed; nothing open.

## Pre-flight

| check | value |
|---|---|
| base | `main` at `e7d298c` |
| free disk on `/` | 15G (floor 10G) |
| build dirs from previous waves | none |
| N | 2 (one Rust unit, one web unit) |

## Units

| unit | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|
| story:server-skeleton | `impl/server-skeleton` | `~/.local/state/worktree/trees/b10x/repoview/impl-server-skeleton` | `~/.cache/b10x-target/repoview-server-skeleton` | `~/.cache/repoview-wave-one/server-skeleton/scratch` | planned |
| story:web-skeleton | `impl/web-skeleton` | `~/.local/state/worktree/trees/b10x/repoview/impl-web-skeleton` | `web/node_modules` inside the worktree | `~/.cache/repoview-wave-one/web-skeleton/scratch` | planned |
| integration | `wave/one` | `~/.local/state/worktree/trees/b10x/repoview/wave-one` | `~/.cache/b10x-target/repoview-wave-one` | `~/.cache/repoview-wave-one/coordinator` | opening commit |

Agents: `aep:implementor` per unit, then `aep:adversary` per unit.
