---
format: aep.planning-md/3
id: epic:repository
kind: epic
status: draft
title: Git state and top-level documents
summary: Branch, dirty files, commits, tags, worktrees, and README, AGENTS.md, STATUS.md, CHANGELOG rendered.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- depends_on: epic:shell
revision: 1
---
## Outcome

The project's Git state and its top-level documents are readable in one page.

## Scope

- Source `vcs`: `git status --porcelain=v2 --branch`, `git log` (last 50, with a parseable
  `--format`), `git tag --sort=-creatordate`, `git remote -v`, `git worktree list --porcelain`.
- Source `docs`: `README.md`, `AGENTS.md`, `STATUS.md`, `CHANGELOG.md`, `Taskfile.yml` tasks,
  workspace members from `Cargo.toml`, `package.json` scripts.
- Repository page: branch, upstream ahead/behind, dirty files, commits, tags, remotes, worktrees;
  documents rendered from markdown (sanitised).

## Acceptance

- With one modified file, the page lists it with the same status letter `git status --short`
  prints.
- `AGENTS.md` and `README.md` render as separate tabs; a missing one shows "absent".

## Depends on

`epic:shell`.
