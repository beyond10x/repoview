---
format: aep.planning-md/3
id: review-result:adversary-repository-page-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:repository-page
relations:
- reviews: story:repository-page
revision: 1
---
unit: story:repository-page, uncommitted working tree ~/.local/state/worktree/trees/b10x/repoview/impl-repository-page (base 59771f6)
verdict: NEEDS-CHANGE
cases: executed 109→110 Rust (109 = the run below minus my one case), 147 web; red 2 (my case, plus the known `api_repository_served`)
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths under ~/.cache/repoview-wave-two/repository-page/scratch/adv2, the assigned build dir, and a Docker image `ubuntu:22.04` (pulled, then removed)
needs-coordinator: none

The three round-1 corrections hold. All 3 pass-1 cases are green now. I found one new defect: `/api/vcs` fails as a whole on Git older than 2.36, which includes the 2.34.1 that Ubuntu 22.04 ships.

**1. Diff stat.** Only the unit's 2 tracked files (`api/repository.rs`, `RepositoryPage.vue`). I added `crates/repoview/tests/adversary_repository_page_pass2.rs`; no implementation file changed.

**2. Case added.** `vcs_answers_on_a_git_without_worktree_list_z` puts a `git` stub on `PATH` that refuses `worktree list … -z` the way 2.34.1 does and hands every other call to the real git; expects 200 with `head`, 1 commit and the root worktree. Red:
```
panicked at crates/repoview/tests/adversary_repository_page_pass2.rs:96:5:
assertion `left == right` failed: {"availability":"Failed","diagnostic":"error: unknown switch `z'\n","tool":"git",...}
  left: 503
 right: 200
```
Checked in a `ubuntu:22.04` container: `git version 2.34.1`; `git worktree list --porcelain -z` printed "error: unknown switch `z'" and exited 129; `status`, `log`, `tag`, `rev-parse --absolute-git-dir` with the route's arguments exited 0.

**3. Gates:** `cargo test … --no-fail-fast` EXIT=101, 108 passed, 2 failed (my case and `api_repository_served`); fmt 0; clippy 0; `pnpm check` "Tests 147 passed (147)", EXIT=0.

**4. Findings**

| # | file:line | verdict / origin | what reaches it | fix |
|---|---|---|---|---|
| 1 | `crates/repoview/src/api/repository.rs:163` | NEEDS-CHANGE / introduced | Git below 2.36 (Ubuntu 22.04 LTS 2.34.1, Debian 11 2.30); the `?` on the worktree call makes the whole route 503 | Fall back to `--porcelain` without `-z` on exit 129, or let a worktree failure empty that field only |
| 2 | `crates/repoview/src/api/repository.rs:146` | INFEASIBLE / pre-existing | A `filter.<x>.clean` in `.git/config` used by `.gitattributes` runs on every `git status` (also in the base vcs source); a clone cannot carry `.git/config` | None in this unit |

**5. Attacked and held:** pass-1 assertions unchanged (only the tasks case adapted); fixed-literal names opened directly in the canonicalized root; git-dir check for bare repo, `.git/worktrees/x`, `.git/modules/x`, linked worktree `.git` file, `--separate-git-dir`; `GIT_DIR` inheritance pre-existing in repoview-sources (reached only when started from a hook); crafted log objects do not shift fields; new subprocesses run no hooks, pager, gpg or fsmonitor; page has no task list and no `/api/tasks` request.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/repository-page/scratch/adv2/{nul,filt,case-red.log,suite.log,web.log}`, the assigned build dir, Docker image `ubuntu:22.04` pulled then removed; lease `adversary-repository-page-pass-2` taken and released.

```findings
- file: crates/repoview/src/api/repository.rs
  line: 163
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`git worktree list --porcelain -z` needs Git 2.36; on Ubuntu 22.04's 2.34.1 it exits 129 and the `?` turns the whole /api/vcs into a 503"
- file: crates/repoview/src/api/repository.rs
  line: 146
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "git status runs a clean filter defined in .git/config on every request; only a .git the user did not write reaches it, and the base vcs source does the same"
```
