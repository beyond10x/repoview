---
format: aep.planning-md/3
id: review-result:adversary-quality-page-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:quality-page
relations:
- reviews: story:quality-page
revision: 1
---
unit: story:quality-page, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-quality-page (base 59771f6, uncommitted)
verdict: NEEDS-CHANGE
cases: executed 100→102 Rust, 134→135 web; red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none

**Result:** 3 cases I added fail on 3 defects. 2 of them can happen in normal use (warning); 1 needs input that no current codegate produces (note).

## 1. Diff stat

`git --no-pager diff --stat` shows only the implementer's 2 modified files (`quality.rs` +307, `QualityPage.vue` +84). My additions are 2 untracked test files: `crates/repoview/tests/adversary_quality.rs`, `web/src/components/quality/LanguageCard.adversary.test.ts`. No implementation file was touched.

## 2. Cases added (each run alone first, red now)

| Case | Asserts | Red output when first run |
|---|---|---|
| `adversary_quality.rs::a_first_request_abandoned_mid_start_does_not_start_a_second_run` | A first request dropped during `codegate capabilities`, then a second request, gives 1 capabilities call and 1 assess | `left: (2, 2) right: (1, 1)`, calls `["capabilities ", "capabilities ", "--root markdown", "--root markdown"]` |
| `adversary_quality.rs::stopping_the_server_stops_a_running_assessment` | After SIGTERM, repoview exits and the `codegate assess` child is gone too | `codegate assess (pid 29838) is still running after repoview exited` (the test then killed it) |
| `LanguageCard.adversary.test.ts` | A top finding with `"location": null` still renders its title and shows no location | `TypeError: Cannot read properties of null (reading 'uri')` at `findingLocation src/api/quality.ts:150:49` |

## 3. Gates (run after the cases existed)

| Command | Summary | Exit |
|---|---|---|
| `cargo test -p repoview -p repoview-sources --locked --no-fail-fast` | `test result: FAILED. 0 passed; 2 failed` (adversary_quality); every other binary ok; 102 cases in total | 101 |
| `cd web && pnpm check` | prettier, eslint and vue-tsc passed; `Tests  1 failed \| 134 passed (135)` | 1 |
| `cargo fmt -p repoview -p repoview-sources -- --check` | clean | 0 |

## 4. Findings

| # | file:line | Verdict | Origin | What reaches it |
|---|---|---|---|---|
| F1 | `crates/repoview/src/api/quality.rs:300` | NEEDS-CHANGE | introduced | Reloading or leaving the page while the first `/api/quality` is still running (snapshot + capabilities + `git ls-files`, up to 10 s each). |
| F2 | `crates/repoview/src/api/quality.rs:179` | NEEDS-CHANGE | introduced | Ctrl-C or SIGTERM on `repoview open` while an assessment runs. |
| F3 | `web/src/api/quality.ts:150` | INFEASIBLE | introduced | Nothing found. |

F1: Hyper drops the handler future when the client disconnects; `OnceCell::get_or_try_init` releases the cell to the next caller, but the detached `spawn_blocking(Assessments::start)` keeps going. Fix: start the run in a task a dropped request does not cancel, or a sync `OnceLock` inside the blocking closure.
F2: `run` puts each child in its own process group (`repoview-sources/src/process.rs:64`), so neither Ctrl-C nor repoview's exit reaches it, and the 120 s timeout dies with the server. Fix: keep the running children's group ids and SIGKILL them on shutdown.
F3: the installed Go codegate never emits `null` for location (`Location ... omitempty`). Fix: treat `location == null` like `undefined`.

Not tested, judgement only: a running page keeps polling every 2 s after a permanent 403 or once the server is gone.

## 5. Attacked and could not break

Flag injection (capabilities names never reach argv; argv uses `&'static str` from `MARKERS`); concurrent first requests serialize on the OnceCell; hung run bounded at 120 s; codegate removed mid-run becomes `failed`; memory one slot per token; scores negative/over-max/`score_max` 0/missing/strings clamped or dropped; never a score for an unassessed language (gated on `status === 'assessed'`); no `v-html`; polling stops on unmount.

## 6. Paths written outside the worktree

None, apart from the assigned build dir.

## 7. Findings block

```findings
- file: crates/repoview/src/api/quality.rs
  line: 300
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a first /api/quality request dropped by the client while Assessments::start runs leaves its blocking start running, and the next request starts a second capabilities call and a second assess per language"
- file: crates/repoview/src/api/quality.rs
  line: 179
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: background codegate assess children survive repoview's SIGTERM or Ctrl-C because they lead their own process group and nothing kills them on shutdown, so the 120 s bound no longer holds
- file: web/src/api/quality.ts
  line: 150
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "findingLocation throws on a finding with location null and the card fails to render; no current codegate emits null there"
```
