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

## Close (2026-10-05)

| unit | commit | adversary passes | findings | tests |
|---|---|---|---|---|
| story:server-skeleton | `3dc32bf` | 2 | 9 then 7, 0 carried; 16 fixed (1 recorded as a wire decision) | 77 |
| story:web-skeleton | `d39355c` | 2 | 3 then 2, 0 carried; 5 fixed | 63 |

The correction after each second pass was reviewed by the coordinator: adversary files unchanged, no
`#[ignore]`/`.skip`, gates re-run in the unit trees.

Gate on `wave/one` at `40984a7`, one exit per step: `aep plan artifact validate` 0 (valid), `ess
specify validate --path ess` 0, `cargo fmt --check` 0, `cargo clippy -D warnings` 0, `cargo test
--workspace` 0 (77 passed, 12 binaries, `--list` 77), `pnpm install --frozen-lockfile` 0,
`pnpm --dir web check` 0 (63 passed). `task build` 0, release binary 3,369,344 bytes.

Integration (epic:shell): the release binary on this tree served `/` 200 `text/html`,
`/plan/x` with `<base href="/">`, `/api/snapshot` 200 with the token; `repoview open` launched Brave
through a 0600 redirect page in `$XDG_RUNTIME_DIR/repoview`; the operator saw the five cards.

The integration gate reused the server unit's build directory after that unit finished; the test
list count (77) matches this tree.

Sub-agent cost (tokens / tool uses / wall): server implementor 127,663 + 173,464 + 217,913 /
56 + 25 + 106 / 644 s + 316 s + 325 s; web implementor 98,465 + 120,478 + 152,871 / 57 + 16 + 26 /
502 s + 161 s + 217 s; adversaries 120,605 / 32 / 407 s, 132,423 / 42 / 672 s, 79,451 / 21 /
238 s, 96,848 / 39 / 308 s; plan critics 8 runs, 33,720 to 43,607 each.

The web pass-1 record was recreated under the same id before its first commit, with the two
home-directory paths it quoted written as `~` (the bot refused the commit on `personal-paths`).
