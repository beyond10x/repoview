---
format: aep.planning-md/3
id: story:plan-pages
kind: story
status: implemented
title: Plan board, tree and artifact pages
summary: aep JSON behind /api/plan/*; board columns from aep, vision-to-task tree, artifact page with rendered body.
relations:
- decomposes: epic:plan
- serves: vision:repoview
- depends_on: story:page-frame
scope:
- confidence: cited
  path: crates/repoview-sources/src/plan.rs
- confidence: cited
  path: crates/repoview/src/api/plan.rs
- confidence: cited
  path: crates/repoview/tests/api_plan.rs
- confidence: cited
  path: web/src/api/plan.ts
- confidence: cited
  path: web/src/components/plan/
- confidence: cited
  path: web/src/pages/PlanArtifactPage.vue
- confidence: cited
  path: web/src/pages/PlanBoardPage.vue
- confidence: cited
  path: web/src/pages/PlanTreePage.vue
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 11}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:15Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Story

As an engineer, I browse the project's AEP plan in three views: a board with status columns, a
hierarchy from vision to task, and one artifact with its rendered body, relations, history and the
reasons for its status.

## Acceptance

1. API (all `GET`, token-guarded, JSON passed through from `aep` unchanged except where named):
   | route | `aep` command run in the project root |
   |---|---|
   | `/api/plan/board` | `aep plan artifact board --format json` |
   | `/api/plan/artifacts` | `aep plan artifact list --format json` |
   | `/api/plan/graph` | `aep plan artifact graph --format json` |
   | `/api/plan/validate` | `aep plan artifact validate --format json` |
   | `/api/plan/artifacts/{id}` | `aep plan artifact show {id} --format json` |
   | `/api/plan/artifacts/{id}/history` | `aep plan artifact history {id} --format json` |
   | `/api/plan/artifacts/{id}/explain` | `aep plan artifact explain {id} --format json` |
   `{id}` must match `^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$` (aep's id grammar without `/`; `vision:O2` is valid) or the answer is 400 and no process starts. A
   non-zero `aep` exit is 502 with `{ "tool": "aep", "exit": n, "stderr": "…", "stdout": "…" }` (`validate`
   prints its problems on stdout and exits 1). No `aep` on `PATH`
   is 503 with the same shape. Rust tests use a stub `aep` on a test `PATH` for each case, including
   an id with `;`, `..` as a whole part, a space and a `/`, and `vision:O2` accepted.
2. Board page (`/plan`): one column per entry of `board --format json`, in its order, with the
   column's own status name, and its description when `board --format json` carries one (aep 0.68.0
   does not; nothing is invented in its place); each artifact as a card with kind, id, title. Filter
   box (title, id, and tag where the list output carries tags) and kind filter. Status names and descriptions come only from the board
   output; the web code holds no list of statuses (a vitest case feeds an invented status
   `zz-new` and sees its column).
3. Tree page (`/plan/tree`): a collapsible tree built from `relations` in `artifacts`: children are
   the artifacts whose `serves`, `designs`, `decomposes` or `implements` edge targets the parent.
   Roots are visions; artifacts with none of those edges and no children appear under
   "Unattached". An artifact reachable twice is shown under each parent. Vitest with a fixture of
   this repository's store (`aep plan artifact list --format json`, captured into
   `web/src/__fixtures__/`).
4. Artifact page (`/plan/artifact/:id`): title, kind, status, revision, summary; the body through
   `MarkdownView`; relations both ways as links to their artifact pages; scope; history (oldest
   first); explain output; findings and outcomes when present. A 502 shows the `stderr` text.
5. `validate` result on the Board page header: "valid" or the defect lines verbatim.
6. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Scope

Owns `crates/repoview-sources/src/plan.rs`, `crates/repoview/src/api/plan.rs`,
`crates/repoview/tests/api_plan.rs`, `web/src/pages/PlanBoardPage.vue`, `web/src/pages/PlanTreePage.vue`,
`web/src/pages/PlanArtifactPage.vue`, `web/src/api/plan.ts`, `web/src/components/plan/`,
`web/src/__fixtures__/plan/` and their tests.

## Out of scope

Moves, approvals and any write; `aep plan serve`; caching.


