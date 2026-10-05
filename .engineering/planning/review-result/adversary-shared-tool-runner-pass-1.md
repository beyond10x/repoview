---
format: aep.planning-md/3
id: review-result:adversary-shared-tool-runner-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:shared-tool-runner
relations:
- reviews: story:shared-tool-runner
revision: 1
---
I found no defects. I added no test cases and changed no files. Every caller of `run` gets the same messages it got before, and the plan and spec routes return the same HTTP status and body for every kind of failure as they did at `cfc7afa`.

```
unit: story:shared-tool-runner, uncommitted working tree of ~/.local/state/worktree/trees/b10x/repoview/impl-shared-tool-runner on base cfc7afa
verdict: nothing found
cases: executed 203→203, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths, both logs in the assigned scratch dir
needs-coordinator: none
```

**1. Diff stat.** The implementor's changes only: `repoview-sources/src/lib.rs` (+/-4), `process.rs` (98), `api/plan.rs` (249, mostly deleted), `api/spec.rs` (153, mostly deleted); untracked `crates/repoview-sources/tests/process.rs`.

**2. Cases added:** none.

**3. Gate runs.** `cargo test -p repoview -p repoview-sources --locked`: every `test result: ok.`, 203 passed, 0 failed, exit 0 (own build dir; the 18 `process.rs` names match the file). `cd web && pnpm check`: `Tests 524 passed (524)`, exit 0.

**4. Judgement findings:** none.

**5. Attacked and could not break:** `run` messages (exit code, signal, timeout, spawn error; truncation once); plan HTTP mapping unchanged (200 bytes through; exit 0 without JSON → 502 with `exit:0` and `stdout`; non-zero or signal → 502 with code or `exit: null` and "killed by a signal" plus `stdout`; timeout or spawn error → 502 with `exit: null`, no `stdout`; missing tool → 503); spec validate and view mapping unchanged; `env.root`/`env.path` equivalent to base; `wait_timeout` from several threads (wait-timeout 0.2.1 handles it); one deadline and process-group kill; stdin `Stdio::null()`; no stdout cap existed at base; the read-only git environment on aep/ess children only disables optional locks, fsmonitor and untracked cache (`GIT_CONFIG_COUNT=2` replaces any inherited `GIT_CONFIG_*`, already true for git at base); the extra `status` field is needed; the 3 deleted plan.rs runner tests are replaced by stricter ones in `tests/process.rs`.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/shared-tool-runner/scratch/{adversary-cargo-test.log,adversary-pnpm-check.log}`; lease `adversary-shared-tool-runner` taken and released.

```findings
[]
```
