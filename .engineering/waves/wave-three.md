# Wave three

Coordinator: the repoview session, 2026-10-05. Skill: aep `implementing` 0.19.2, wave mode. The
operator asked to continue with the release epic, the static export and the two follow-ups; no
stage-1 stop. Commits covered: one per unit, the merges into `wave/three`, the closing store commit,
the merge into `main`. Publishing a release is not part of the wave.

## Selection

`aep plan artifact waves --kind story --status draft` (aep 0.68.0):

```
wave 1
  story:ci-binaries
  story:shared-tool-runner
  story:static-export
  story:taskfile-tasks
1 wave(s), 0 collision(s), 0 unassessed
```

The opening commit adds `serde_yaml_ng = "0.10"` (Taskfile reading, workflow test) and moves `tower`
to a normal dependency (static export drives the router in-process), so no unit edits `Cargo.toml`.
The public docs site (`docs` skill) follows once `story:static-export` merges.

## Units

| unit | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|
| story:ci-binaries | `impl/ci-binaries` | `~/.local/state/worktree/trees/b10x/repoview/impl-ci-binaries` | `~/.cache/b10x-target/repoview-ci-binaries` | `~/.cache/repoview-wave-three/ci-binaries/scratch` | dispatched |
| story:static-export | `impl/static-export` | `~/.local/state/worktree/trees/b10x/repoview/impl-static-export` | `~/.cache/b10x-target/repoview-static-export` | `~/.cache/repoview-wave-three/static-export/scratch` | dispatched |
| story:shared-tool-runner | `impl/shared-tool-runner` | `~/.local/state/worktree/trees/b10x/repoview/impl-shared-tool-runner` | `~/.cache/b10x-target/repoview-shared-tool-runner` | `~/.cache/repoview-wave-three/shared-tool-runner/scratch` | dispatched |
| story:taskfile-tasks | `impl/taskfile-tasks` | `~/.local/state/worktree/trees/b10x/repoview/impl-taskfile-tasks` | `~/.cache/b10x-target/repoview-taskfile-tasks` | `~/.cache/repoview-wave-three/taskfile-tasks/scratch` | dispatched |
| integration | `wave/three` | `~/.local/state/worktree/trees/b10x/repoview/wave-three` | `~/.cache/b10x-target/repoview` | `~/.cache/repoview-wave-three/coordinator` | opening commit |

## State at the usage limit (2026-10-05)

- Merged into wave/three: story:shared-tool-runner, story:ci-binaries, story:static-export; coordinator commits: ess/22, web/dist digest embed fix.
- story:taskfile-tasks: committed on impl/taskfile-tasks after correction round 1 (Rust 219, web 542); adversary pass 2 not yet run; not merged.
- Wave four drafted outside the store: ~/.cache/repoview-wave-four/coordinator/story-docs-site.md.
- Next: taskfile pass 2, merge, full gate, close wave three, push; then docs site (wave four) and Stage C/D.
