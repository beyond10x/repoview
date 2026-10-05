---
format: aep.planning-md/3
id: story:shared-tool-runner
kind: story
status: draft
title: One subprocess runner with exit code and output for every module
summary: run_output in repoview-sources; plan and spec API modules drop their private runners.
relations:
- decomposes: epic:shell
- serves: vision:repoview
revision: 1
---
## Story

As a maintainer, one subprocess runner serves every source and API module: it reports the exit code,
stdout and stderr on failure, so `api/plan.rs` and `api/spec.rs` stop carrying their own copies.

## Acceptance

1. `repoview_sources::run_output(env, program, args, timeout)` returns `{ exit: Option<i32>, stdout,
   stderr, timed_out }` with the process-group kill, one deadline and the read-only git environment
   `run` has today; `run` is rebuilt on it with unchanged messages (existing tests unchanged).
2. `api/plan.rs` and `api/spec.rs` use it; their private runners are deleted; their tests pass
   unchanged.
3. The spec root list comes from a public `detected_roots(env)` in `repoview-sources`, so the spec
   API starts no `ess --version` to find roots.

## Notes

The spec unit's verified patch is a starting point:
`~/.cache/repoview-wave-two/spec-pages/scratch/shared-runner-and-roots.patch` (wave two, 2026-10-05).
