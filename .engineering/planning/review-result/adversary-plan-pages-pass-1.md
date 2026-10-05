---
format: aep.planning-md/3
id: review-result:adversary-plan-pages-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:plan-pages
relations:
- reviews: story:plan-pages
revision: 1
---
unit: story:plan-pages, uncommitted working tree in ~/.local/state/worktree/trees/b10x/repoview/impl-plan-pages (base 59771f6)
verdict: NEEDS-CHANGE
cases: executed 271→274 (Rust 94→95, web 177→179), red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths
needs-coordinator: yes. The story's id pattern rejects the ids aep and nine real stores use; only an amendment to the story can fix that (finding 3)

## 1. Diff stat

`git --no-pager diff --stat` lists only the implementor's four tracked files (plan.rs, PlanArtifactPage.vue, PlanBoardPage.vue, PlanTreePage.vue). My three files are new and untracked: `crates/repoview/tests/api_plan_adversary.rs`, `web/src/pages/PlanArtifactPage.adversary.test.ts`, `web/src/components/plan/PlanCard.adversary.test.ts`. A size probe `web/src/zzprobe.test.ts` was written and deleted. No implementation file was touched.

## 2. Cases added (each run alone first, red output captured then)

| File | Asserts | Now |
|---|---|---|
| `PlanCard.adversary.test.ts` | A card fed the `blocked_by` that aep 0.68.0 actually prints, `[{blocker,type}]`, names the blocker | red |
| `PlanArtifactPage.adversary.test.ts` | An explain `blocked_by` in aep's real shape links to `/plan/artifact/<blocker>` | red |
| `api_plan_adversary.rs` | `GET /api/plan/artifacts/vision:O2` (and `story:a_b`) reaches the stub aep and returns 200 | red |

```
Received: "...Foundation source pins blocked by [object Object]"
AssertionError: expected [ '/plan/artifact/[object Object]' ] to include '/plan/artifact/dependency-blocker:met…'
assertion `left == right` failed: vision:O2: {"error":"artifact id must match ^[a-z0-9-]+:[a-z0-9-]+$"}  left: 400 right: 200
```

## 3. Suite runs (after the cases existed)

`cargo test -p repoview -p repoview-sources --locked --no-fail-fast` exited 101: 94 passed, 1 failed (`a_vision_id_as_aep_writes_it_reaches_aep`). `cd web && pnpm check` exited 1: `Tests  2 failed | 177 passed (179)`.

## 4. Findings

| # | file:line | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| 1 | `web/src/components/plan/PlanCard.vue:17` | CONFIRMED / introduced | Prints "blocked by [object Object]". `plan.ts:20` types `blocked_by` as `string[]`, but aep prints `{blocker,type,withholds?}` (aep-cli `Blocking`) | Any blocked artifact on the board; in aep's own store `migration-plan:foundation-source-pins-20260909` comes out exactly like this |
| 2 | `web/src/pages/PlanArtifactPage.vue:173` | CONFIRMED / introduced | Each explain blocker links to `/plan/artifact/[object Object]`; the unit's fixture invented `blocked_by: ['story:page-frame']` | The same artifact's page |
| 3 | `crates/repoview/src/api/plan.rs:95` | NEEDS-CHANGE / introduced | `^[a-z0-9-]+:[a-z0-9-]+$` answers 400 for ids aep accepts (`ArtifactId::new` allows `[A-Za-z0-9._/-]`) | Every vision in the aep, canon, commission, engineering-protocols, entity-runtime, ess, governor, intake and loom stores is `vision:O<n>`; each one's Artifact page answers 400. The code follows acceptance 1 as written |

Suggested: type `blocked_by` as `{blocker,type,withholds?}[]`; amend the pattern to aep's grammar without `/`, e.g. `^[A-Za-z0-9_-][A-Za-z0-9._-]*:[A-Za-z0-9_-][A-Za-z0-9._-]*$`; `artifactRoute` should `encodeURIComponent` the id. Cross-repo targets (`entity-runtime/story:typed-references`) still cannot be linked (note, not raised).

## 5. Attacked and not broken

argv `--` handling; runner deadline kill with a grandchild holding the pipe, both streams drained, non-JSON/partial JSON → 502; list/board/graph on aep's 342-artifact store 38–42 ms, 160–222 KB; tree with self-edges, cycles, missing targets, duplicates (430 nodes in 12 ms); no `v-html` outside MarkdownView; real output field names match. History keys collide on repeated `(revision, at)` (not raised).

## 6. Paths written outside the worktree

`~/.cache/repoview-wave-two/plan-pages/scratch/{adv-cargo.log,adv-web.log,aep-list.json,store/}` and the assigned build dir.

## 7. Findings block

```findings
- file: web/src/components/plan/PlanCard.vue
  line: 17
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "blocked_by from aep 0.68.0 is a list of {blocker,type} objects, so every blocked card reads 'blocked by [object Object]'"
- file: web/src/pages/PlanArtifactPage.vue
  line: 173
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: explain blockers link to /plan/artifact/[object Object] because the unit's fixture invented string blockers that real aep never prints
- file: crates/repoview/src/api/plan.rs
  line: 95
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the acceptance id pattern refuses ids aep accepts, so the Artifact page of every vision:O<n> root in nine beyond10x stores answers 400
```
