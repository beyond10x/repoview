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
