---
format: aep.planning-md/3
id: story:taskfile-tasks
kind: story
status: draft
title: Task list read from Taskfile.yml without executing it
summary: Parse Taskfile.yml in Rust for task names and descriptions; nothing runs.
relations:
- decomposes: epic:repository
- serves: vision:repoview
- depends_on: story:repository-page
revision: 1
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
