---
format: aep.planning-md/3
id: story:repository-page
kind: story
status: implemented
title: Git state, documents and tasks
summary: Status, commits, tags, remotes, worktrees, README/AGENTS/STATUS/CHANGELOG and task list behind /api/vcs, /api/docs, /api/tasks.
relations:
- decomposes: epic:repository
- serves: vision:repoview
- depends_on: story:page-frame
scope:
- confidence: cited
  path: crates/repoview-sources/src/docs.rs
- confidence: cited
  path: crates/repoview-sources/src/vcs.rs
- confidence: cited
  path: crates/repoview/src/api/repository.rs
- confidence: cited
  path: crates/repoview/tests/api_repository.rs
- confidence: cited
  path: web/src/api/repository.ts
- confidence: cited
  path: web/src/components/repository/
- confidence: cited
  path: web/src/pages/RepositoryPage.vue
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:40:02Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-05T13:40:02Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:16Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":2,"review_outcome":3,"verification":1}}}
---
## Story

As an engineer, I read the project's Git state and its top-level documents on one page.

## Acceptance

1. API (token-guarded `GET`):
   | route | answer |
   |---|---|
   | `/api/vcs` | `{ branch, head, upstream, ahead, behind, dirty: [{path, status}], commits: [{sha, author, date, subject}] (last 50), tags: [{name, sha, date}] (newest 30), remotes: [{name, url}], worktrees: [{path, head, branch}] }` from `git status --porcelain=v2 --branch -z`, `git log -50 --format=…`, `git tag --sort=-creatordate --format=…`, `git remote -v`, `git worktree list --porcelain`, all with the read-only git environment the `vcs` source already uses |
   | `/api/docs` | `[{ "name": "README.md", "present": bool }]` for README.md, AGENTS.md, STATUS.md, CHANGELOG.md |
   | `/api/docs/{name}` | `{ "name", "markdown": "<file content>" }`; `{name}` must be one of those four, else 404; files over 1 MiB are 413 |
   Remote URLs have any `user:password@` or token userinfo removed before they leave the server
   (test with `https://x-access-token:abc@github.com/o/r.git`).
2. Repository page (`/repository`): a status block (branch or "detached HEAD", head, upstream with
   ahead/behind, dirty files with their status letters), commits table, tags, remotes, worktrees;
   a tab per document present, rendered through `MarkdownView`, absent ones listed as "absent"; the
   task list with descriptions when available.
3. Rust tests build real Git repositories with `tempfile` (detached HEAD, unborn branch, an upstream
   ahead/behind, a dirty file of each porcelain status, a linked worktree) and assert the JSON;
   `.git/index` is unchanged after every request.
4. Vitest covers each block with fixtures, including detached HEAD and an empty repository.
5. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Scope

Owns `crates/repoview-sources/src/vcs.rs`, `crates/repoview-sources/src/docs.rs`,
`crates/repoview/src/api/repository.rs`, `crates/repoview/tests/api_repository.rs`,
`web/src/pages/RepositoryPage.vue`, `web/src/api/repository.ts`, `web/src/components/repository/`,
`web/src/__fixtures__/repository/` and their tests.

## Out of scope

Diffs, blame, file browsing; any write.


## Decision 2026-10-05: no task list

`task --list-all --json` evaluates a Taskfile's `vars: sh:` commands when it declares `dotenv:`
(`review-result:adversary-repository-page-pass-1`, finding 1), so listing tasks through the `task` CLI
runs a cloned repository's shell code. `/api/tasks` and the task list are removed from this story.
Reading `Taskfile.yml` without executing it is a later story.
