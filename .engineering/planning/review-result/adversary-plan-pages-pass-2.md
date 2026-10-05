---
format: aep.planning-md/3
id: review-result:adversary-plan-pages-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:plan-pages
relations:
- reviews: story:plan-pages
revision: 1
---
```
unit: story:plan-pages, uncommitted working tree ~/.local/state/worktree/trees/b10x/repoview/impl-plan-pages (base 59771f6), after correction round 1
verdict: CONFIRMED (1 warning; the four corrections hold)
cases: executed 280→282 (Rust 95→95, web 185→187), red 3 (1 new; 2 are the known plan.client.test.ts cases)
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 paths under ~/.cache/repoview-wave-two/plan-pages/scratch, plus the assigned build dir
needs-coordinator: none
```

## 1. Diff stat

Only the implementor's 4 tracked files: `plan.rs` (+371), `PlanArtifactPage.vue` (+472), `PlanBoardPage.vue` (+220), `PlanTreePage.vue` (+102). I added `web/src/pages/PlanArtifactPage.adversary2.test.ts`; no implementation file changed.

## 2. Cases added

| Case | Asserts | Now |
|---|---|---|
| `shows each record a move rested on, as text` | explain step `rested_on` records show kind and source, never `[object Object]` (real records from repoview's `story:server-skeleton`) | green |
| `names what executed a move when that was not its actor` | the `executor` aep prints on a step (real data, codegate `epic:offline-dependency-evaluation`) appears in "Why this status" | **red** |

```
AssertionError: expected 'Why this statusReacheddraft → propose…' to contain 'agent:codegate-wave001'
Received: "…active → implemented2026-10-02T12:55:18Zno record: nothing was recorded about how this was decidedNext"
Tests  1 failed | 1 passed (2)
```

Mutant probe: `PlanArtifactPage.vue:190` `v-if` on `rested_on` set to `false` — the unit's own 15 cases stayed green; only case 1 caught it.

## 3. Suite runs

`cargo test … --no-fail-fast` EXIT=0, 95 passed. `pnpm check` EXIT=1, `Tests  3 failed | 184 passed (187)` (2 known `plan.client.test.ts`, 1 above). Without my file: `Tests  2 failed | 183 passed (185)`.

## 4. Findings

| # | file:line | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| 1 | `web/src/pages/PlanArtifactPage.vue:186` | CONFIRMED / introduced | `reached` steps show from → to, `at` and records but drop `executor` and `correlation`, which aep's text output prints as "executed by …, correlation …" | Real explain output: 83 steps carry `executor` and 65 carry `correlation` across codegate, ess, metaharness and others |

Suggested fix: show `step.executor` and `step.correlation` next to `step.at` when present.

## 5. Attacked, not broken

Pass-1 files unchanged and green; all 4751 artifact ids in the 44 stores under ~/beyond10x/*/.engineering/planning match the pattern (upper case and `.` accepted); of 1322 relation targets and blockers only `entity-runtime/story:typed-references` is refused (`/`, noted in pass 1); shape guards pass on `list`, `board`, `show`/`history`/`explain` for all 3623 artifacts in the 21 stores aep 0.68.0 opens; no duplicate `v-for` keys; `rested_on`/`recorded_since` always aep's `Admitted` type; every allowed character round-trips through vue-router, `encodePath` and axum `Path`; invalid-store validate matches `validation()`.

## 6. Paths written outside the worktree

~/.cache/repoview-wave-two/plan-pages/scratch/{p2-ids.txt,p2-targets.txt,p2-out/,p2-collect.sh,p2-collect.log,p2-plan-copy.ts,p2-probe.ts,p2-probe2.ts,p2-invalid.out,p2-invalid.err,p2-cargo.log,p2-web.log} (store and web copies deleted), the assigned build dir; lease `adversary-plan-pages-pass-2` released.

```findings
- file: web/src/pages/PlanArtifactPage.vue
  line: 186
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "explain steps drop executor and correlation, so the 83 real moves aep reports as executed by an agent show no executor on the Artifact page"
```
