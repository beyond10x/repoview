---
format: aep.planning-md/3
id: story:taskfile-tasks
kind: story
status: active
title: Task list read from Taskfile.yml without executing it
summary: Parse Taskfile.yml in Rust for task names and descriptions; nothing runs.
relations:
- decomposes: epic:repository
- serves: vision:repoview
- depends_on: story:repository-page
scope:
- confidence: cited
  path: crates/repoview/src/api/repository.rs
- confidence: cited
  path: crates/repoview/tests/api_tasks.rs
- confidence: cited
  path: web/src/api/repository.ts
- confidence: cited
  path: web/src/components/repository/
- confidence: cited
  path: web/src/pages/RepositoryPage.vue
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T16:15:10Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-05T16:15:10Z", actor: "human:timo", revision: 8}
---
## Story

As an engineer, I see the project's tasks (name and description) on the Repository page, read from
`Taskfile.yml` without running anything.

## Acceptance

1. `/api/tasks` reads `Taskfile.yml` (and its `includes:` files inside the project) with a YAML
   parser in Rust and returns `[{ name, desc, internal }]`; no process starts (a test with a
   `vars: sh:` probe and `dotenv:` asserts the probe file is not created).
2. The Repository page shows the list; absent Taskfile shows "absent".

## Notes

Running `task --list-all --json` was removed in wave two because it evaluates `vars: sh:` when the
Taskfile declares `dotenv:` (`review-result:adversary-repository-page-pass-1`, finding 1).


## Left wave three (2026-10-06)

Two adversary passes found 4 then 6 defects, none carried: each structural cap (node budget, nesting
pre-scan, file and entry caps) moved the attack to a new resource (scalar bytes, quoted closers,
empty includes, response size, post-budget parsing). Records: `review-result:adversary-taskfile-tasks-pass-1`,
`-pass-2`. The work is on branch `impl/taskfile-tasks` (0be2ff5), not merged; `main` has no task list.

Next attempt, decided by the coordinator: one per-request budget that every cost draws from —
bytes read from disk, bytes of parsed scalars (aliases charged per use), nodes, include visits and
response bytes — plus a wall-clock deadline that aborts parsing (parse on a worker thread and drop
it), and a pre-scan that tracks quotes and comments. The six pass-2 cases in
`crates/repoview/tests/adversary_taskfile_tasks_pass_2.rs` on that branch are the acceptance floor.
