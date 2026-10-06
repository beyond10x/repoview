# Wave four

Coordinator: the repoview session, 2026-10-06. Skill: aep `implementing` 0.19.2, wave mode. The
operator asked to continue with the docs site and the static export of repoview; no stage-1 stop.
Commits covered: the unit commit, its merge into `wave/four`, the closing store commit, the merge
into `main`. Stage C (Atlas) and Stage D (live checks) follow the merge, as the `docs` skill says.

## Units

| unit | branch | worktree | build dir | scratch |
|---|---|---|---|---|
| story:docs-site | `impl/docs-site` | `~/.local/state/worktree/trees/b10x/repoview/impl-docs-site` | `~/.cache/b10x-target/repoview-docs-site` | `~/.cache/repoview-wave-four/docs-site/scratch` |

N is one: the docs site touches the workspace manifest, the Taskfile and two workflows, which no
other ready story can share.
