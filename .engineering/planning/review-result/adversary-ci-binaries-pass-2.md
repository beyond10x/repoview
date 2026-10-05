---
format: aep.planning-md/3
id: review-result:adversary-ci-binaries-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:ci-binaries
relations:
- reviews: story:ci-binaries
revision: 1
---
The corrections hold. I found no defect in either workflow file, so I added no case. Two notes concern the new test's own matching.

```
unit: story:ci-binaries, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-ci-binaries (base cfc7afa, 3 untracked files)
verdict: CONFIRMED (2 notes)
cases: executed 202→202, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths, all in the assigned scratch dir (part 6)
needs-coordinator: none
```

**1. Diff stat:** `git status --short` shows only the unit's three untracked files; I edited nothing.

**2. Cases added:** none.

**3. Gate runs:** `--test workflows` `test result: ok. 14 passed; 0 failed`, EXIT=0 (all 14 names in this tree); full `cargo test --workspace --locked --no-fail-fast` 202 passed, 0 failed, EXIT=0. The pass-1 case is green with the same panic text and the same `fn`-to-`assert!` offset (409→420 then, 475→486 now); byte identity cannot be proven.

**4. Findings**

| # | file:line | finding | verdict / origin | what reaches it |
|---|---|---|---|---|
| 1 | `crates/repoview/tests/workflows.rs:342,346` | The swallow check misses common forms; scratch mutations stayed green (14/14): `task check ||true`, `task check \|\| exit 0`, `"$work/snapshot.json" \|\| :`, `set +o errexit`, a second `\|\| true` after `;` on the `trap` line. Control `\|\| true` goes red | CONFIRMED / introduced, note | Only a future edit to a workflow |
| 2 | `crates/repoview/tests/workflows.rs:303` | The doc comment says a `shell` override "would drop `bash -eo pipefail`"; GitHub's default is `bash -e {0}` without pipefail (beyond10x/aep run 37315597186 log). The test rejects `shell: bash`, the only setting that adds pipefail. No current pipe depends on pipefail | CONFIRMED / introduced, note | A later edit adding a pipe based on this comment |

**5. Attacked and could not break:** PR group `refs/pull/N/merge` per PR; push group per commit; no other trigger evaluates the expression; `cancel-in-progress` expression evaluates to a boolean; the strict test forbids nothing needed (three main-push steps match `MAIN_PUSH`; `checksums` uses `needs:`; `trap` line exempt). Untested but harmless: a per-sha group for PRs would pass 14/14 (costs CI minutes only).

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/ci-binaries/scratch/{pass2-probe/,pass2-workflows.log,pass2-suite.log}` (probe `target/` deleted).

```findings
- file: crates/repoview/tests/workflows.rs
  line: 346
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the swallow check stays green for ||true, || exit 0, || : inside a block scalar, set +o errexit, and a second swallow appended to the trap line"
- file: crates/repoview/tests/workflows.rs
  line: 303
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the doc comment claims the default shell has pipefail, but GitHub's default is bash -e {0} and the test rejects shell: bash, the only setting that adds pipefail"
```
