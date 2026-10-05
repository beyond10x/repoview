---
format: aep.planning-md/3
id: review-result:adversary-quality-page-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:quality-page
relations:
- reviews: story:quality-page
revision: 1
---
Pass 2 found one defect: `shutdown()` and the exit hook miss assessments that start while the server is stopping. Pass 1's corrections hold.

```
unit: story:quality-page, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-quality-page (base 59771f6, uncommitted)
verdict: NEEDS-CHANGE
cases: executed 104→106 Rust, 138→138 web; red 2 (with e309fed main.rs overlaid; 4 in this tree, 2 of them from the missing Extension<Env> layer only)
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (1 build dir outside the assigned pair, 4 under the assigned scratch)
needs-coordinator: none
```

## 1. Diff stat

`crates/repoview/src/api/quality.rs | 441 +++`, `web/src/pages/QualityPage.vue | 84 +++` (the implementer's). I added `crates/repoview/tests/adversary_quality_pass2.rs`; no implementation file changed.

## 2. Cases added (each run alone first)

| Case | What it checks | Red output when first run |
|---|---|---|
| `shutdown_right_after_start_stops_every_assessment_it_began` | `Assessments::start` (5 languages), then `shutdown()` at once; after 1.5 s no `codegate assess` may run | `codegate assess still running after shutdown(): pids [315512, 315510, 315511, 315515, 315516]` (`adversary_quality_pass2.rs:110`); red in 2 of 2 runs |
| `sigterm_while_the_run_starts_leaves_no_assessment_running` | served binary, 6 attempts, SIGTERM while the first request is inside `codegate capabilities`; no assess may run after exit | overlay copy: `codegate assess left running after repoview exited (attempt, pids): [(0, [329139])]`; red in 5 of 5 runs, orphans in 8 of 30 attempts. In this tree it fails earlier at `:144` (Extension<Env> missing) |

## 3. Suite runs

This tree: 106 executed, red `adversary_quality` 2 (Extension<Env> only) and `adversary_quality_pass2` 2, exit 101. Overlay: `passed 104 failed 2` (my 2 only), exit 101. `pnpm check`: `Tests 138 passed (138)`, exit 0.

## 4. Findings

| # | file:line | Verdict | Origin | What reaches it |
|---|---|---|---|---|
| F1 | `crates/repoview/src/api/quality.rs:265` | NEEDS-CHANGE | introduced | Ctrl-C or SIGTERM on `repoview open` while the first `/api/quality` is inside `codegate capabilities` |

A run is registered only after `Command::spawn` returns (`:330-334`), and `shutdown()` does not stop later spawns; after SIGTERM the graceful drain waits for the starting request, `main` returns, the exit hook runs while assess threads still fork. Fix: a shutting-down flag under the `RUNNING` lock; no spawn once set; after a spawn, kill the new group instead of registering it.

Pass-1 corrections hold (F1, F2 except this window, F3); no weakened assertion.

## 5. Attacked and could not break

PID reuse (a live process group id is not reissued; deregistered after reap); atexit registered once, runs on return, `process::exit`, main-thread panic; lock poisoning recovered; one OnceLock slot per run, panicking `start` leaves it empty; accessors with `null`, arrays, strings, `__proto__`, empty `score_max`, non-finite, unknown statuses. Removing `Registered`'s `Drop` leaves the suite green (cannot be observed without forced PID reuse).

## 6. Paths written outside the worktree

`~/.cache/b10x-target/repoview-quality-page-adv2` (471M), `~/.cache/repoview-wave-two/quality-page/scratch/{adv2-overlay,adv2-suite.log,adv2-web.log,adv2-overlay-suite.log}`.

```findings
- file: crates/repoview/src/api/quality.rs
  line: 265
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "shutdown() and its atexit hook are no barrier: a codegate assess spawned while or just before shutdown runs is registered too late or not at all, so a SIGTERM during the first request's capabilities call leaves orphaned assess processes (8 of 30 served attempts)"
```
