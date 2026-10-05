---
format: aep.planning-md/3
id: review-result:adversary-ci-binaries-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:ci-binaries
relations:
- reviews: story:ci-binaries
revision: 1
---
I found one real defect in `ci.yml`, and the case I wrote for it fails. Separately, the workflows should run on all three runners: nothing I checked would break them on ubuntu-22.04, ubuntu-22.04-arm or macos-14.

```
unit: story:ci-binaries, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-ci-binaries (base cfc7afa, 3 untracked files)
verdict: CONFIRMED (1 warning, 1 note)
cases: executed 200→201, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths, all in the assigned scratch dir (listed in part 6)
needs-coordinator: none
```

**1. Diff stat.** All three unit files are untracked. My only edit is one new case in `crates/repoview/tests/workflows.rs` (lines 405–421); no workflow file changed.

**2. Case added:** `adversary_ci_concurrency_never_drops_a_main_push_build` (`workflows.rs:409`), red:
```
test adversary_ci_concurrency_never_drops_a_main_push_build ... FAILED
panicked at crates/repoview/tests/workflows.rs:420:5:
ci.yml concurrency group `ci-${{ github.event_name }}-${{ github.ref }}` is the same for consecutive pushes to main: a third push cancels the second push's pending run, and that push never uploads its build
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out
EXIT=101
```

**3. Gates:** `--test workflows` 12 passed, 1 failed, exit 101; full `cargo test … --no-fail-fast` 200 passed, 1 failed (same case), exit 101; `cargo fmt -p repoview -- --check` clean.

**4. Findings**

| # | file:line | finding | verdict / origin | what reaches it |
|---|---|---|---|---|
| 1 | `.github/workflows/ci.yml:11-13` | All pushes to `main` share one concurrency group; GitHub keeps one running and one pending per group and cancels the pending run when a newer one queues, whatever `cancel-in-progress` says; that push never uploads its build. Fix: `group: ci-${{ github.event_name == 'pull_request' && github.ref \|\| github.sha }}` | CONFIRMED / introduced, warning | Three pushes to `main` within one CI run's time |
| 2 | `crates/repoview/tests/workflows.rs:498-507` | The smoke-step test stays green if `--exit-status` is dropped from `jq` (jq exits 0 printing `false`), and allows `continue-on-error` or `if:` on gate and smoke steps | CONFIRMED / introduced, note | Only an edit to the workflow |

**5. Checked and found sound:** aep 0.68.0 / ess 0.53.0 asset names and SHA-256s match release digests (aarch64 and darwin present; `<name>/<bin>` layout); go-task v3.53.1 checksums match; `shasum -a 256 --check` exits 1 on wrong or empty digest; all six pinned SHAs are real commits (five match tags; the dtolnay pin is signed, parent on `master`, same as codegate); arm image has jq, curl, git, rustup, Node; Rust 1.98.1 exists; pnpm 11.9.0 needs Node ≥22.13 (setup-node `22`); repoview is public so `ubuntu-22.04-arm` is available; scripts work under macOS bash 3.2; release build runs `task build` (web before cargo); smoke script run locally against the release binary with tag `0.1.0` exits 0, server killed by `trap`; `--version` prints `repoview <Cargo version>`; `task check` validations pass in a shallow clone; `GITHUB_REF_NAME` reaches scripts only through quoted env vars.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/ci-binaries/scratch/{smoke.sh,smoke/,suite.log}`; a shallow clone was created and deleted.

```findings
- file: .github/workflows/ci.yml
  line: 11
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the concurrency group ci-${{ github.event_name }}-${{ github.ref }} is shared by every main push, so a third queued push cancels the pending second run and that push never uploads its Linux build"
- file: crates/repoview/tests/workflows.rs
  line: 498
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the smoke-step test stays green if --exit-status is dropped from jq (jq then exits 0 on false) or if continue-on-error or an if is added to the smoke or gate step
```
