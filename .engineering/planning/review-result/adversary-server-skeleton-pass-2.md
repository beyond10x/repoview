---
format: aep.planning-md/3
id: review-result:adversary-server-skeleton-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:server-skeleton
relations:
- reviews: story:server-skeleton
revision: 1
---
I broke story:server-skeleton again: the pass-1 corrections hold, but 5 new test cases fail and 3 defects need a change before the unit merges. I edited no implementation file and made no commits.

```
unit: story:server-skeleton, uncommitted working tree on base d411e1c (worktree impl-server-skeleton), after correction round 1
verdict: NEEDS-CHANGE
cases: executed 64→69, red 5
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 10 (scratch logs, listed in part 6)
needs-coordinator: none
```

**1. What I touched.** `git --no-pager diff --stat` is empty because the whole unit is untracked. I added two test files, `crates/repoview-sources/tests/adversary_pass2.rs` and `crates/repoview/tests/adversary_pass2_cli.rs`. I also created two probe test files, ran each once, and deleted them (part 5). There is no non-test path.

**Pass-1 cases.** All 6 are green in the suite run below. The three files were last modified at 14:54, before the pass-1 record. Their assertions match the ones quoted in pass 1, so none was weakened.

**2. Cases added.** Each was red on its first run, alone, before the suite. Home paths in the output are written as `~`. After those runs, rustfmt moved the assert lines in `adversary_pass2.rs` from :69 to :72 and from :103 to :109; a rerun was still red at the new lines.

| file | test | red output (verbatim) |
|---|---|---|
| `adversary_pass2_cli.rs` | `sigint_exits_while_a_client_holds_a_half_sent_request` | `repoview open still running 5 s after two SIGINTs while a client held a half-sent request` |
| `adversary_pass2.rs` | `snapshot_does_not_start_a_fsmonitor_daemon_in_the_project` | `the snapshot started git fsmonitor--daemon, which wrote into ~/.cache/claude-tmp/.tmpCdkYgF/.git` |
| `adversary_pass2.rs` | `spec_outside_git_finds_a_shallow_marker_beside_a_large_directory` | `…/bulk-b/system.yaml missed after …/bulk-a was walked first: Section { source_id: "spec", … availability: Absent …` |
| `adversary_pass2_cli.rs` | `sighup_does_not_leave_the_redirect_page_behind` | `redirect page with the run token left behind after SIGHUP: ~/.cache/claude-tmp/.tmpr0moqk/repoview/open-3317856-7ad07732ce59a1ae.html` |
| `adversary_pass2_cli.rs` | `redirect_page_is_not_written_into_the_project_through_a_symlinked_cache` | `repoview wrote ~/.cache/claude-tmp/.tmpuecQng/cache/repoview inside the project (opener got Some("file://~/.cache/claude-tmp/.tmpgcesx4/.cache/repoview/open-…html"))` |

Supporting probes:
- **Shutdown, measured directly.** A client holding a half-sent request kept the server alive for at least 45 s after SIGINT. An idle connection let it exit in 1 s.
- **fsmonitor.** With `core.fsmonitor=true` and both `GIT_OPTIONAL_LOCKS=0` and `--no-optional-locks`, `git status` still started `git fsmonitor--daemon run --detach`. The daemon ran in its own session. I stopped it afterwards.

**3. Suite run** (after the cases existed):
- `cargo test -p repoview -p repoview-sources --locked --no-fail-fast` exited 101 with 5 failed (all mine) and 64 passed.
- The 64 matches the implementor's round-1 green log: 4+0+2+2+1+18+11+2+2+22.
- `cargo clippy -p repoview -p repoview-sources --all-targets --locked -- -D warnings` exited 0.
- `cargo fmt -p repoview -p repoview-sources -- --check` exited 0, after I ran rustfmt on my two files only.

**4. Findings** (they cover the uncommitted tree on d411e1c, which has no `crates/`, so every finding is introduced)

| # | file:line | finding | verdict / severity | what reaches it | fix (named, not applied) |
|---|---|---|---|---|---|
| 1 | `repoview/src/main.rs:173` | Graceful shutdown waits forever for a half-received request. Because the signal handler is installed, a second Ctrl-C does nothing. Before this round, SIGINT simply ended the process. | NEEDS-CHANGE / warning | Any local process that connects to the printed port and sends a partial request, or a browser socket that stalls mid-request. | After the signal, bound the shutdown (for example a 2–3 s timeout, then exit), or exit on a second signal |
| 2 | `repoview-sources/src/vcs.rs:58` | When the user has `core.fsmonitor=true`, `git status` starts a long-lived `fsmonitor--daemon`. It creates `.git/fsmonitor--daemon.ipc` and `.git/fsmonitor--daemon/`, which breaks "writes nothing inside the project". | NEEDS-CHANGE / warning | A global `core.fsmonitor=true` (users set it for speed) or a Scalar-registered repository. Git 2.55 on Linux starts the daemon. | Pass `-c core.fsmonitor=false` on every git call, or set `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` beside `GIT_OPTIONAL_LOCKS` |
| 3 | `repoview-sources/src/spec.rs:58` | The walk goes depth-first in `read_dir` order. A large sibling directory uses up the 20,000-entry budget, so a `system.yaml` one level below the root is reported `Absent`. | NEEDS-CHANGE / warning | A project outside Git, or `git` missing from `PATH`, or a failing `git ls-files` (for example "dubious ownership"), with a `.venv`, `vendor/` or `dist/` walked first. | Walk breadth-first so shallow markers come first; say in the diagnostic when the walk was cut short |
| 4 | `repoview/src/main.rs:179` | `shutdown_signal` handles SIGINT and SIGTERM but not SIGHUP. Closing the terminal therefore leaves the 0600 page and its token behind. | CONFIRMED / note | Closing the terminal tab that runs default `repoview open`. The token in the file is dead once the process exits, so the cost is one stale file per run. | Add SIGHUP (and SIGQUIT) to `shutdown_signal` |
| 5 | `repoview/src/browser.rs:26` | The "inside the project" check compares paths as written. A `~/.cache` that is a symlink into the project puts `repoview/open-*.html` inside the project. | INFEASIBLE / note | The test builds this case; I found no real setup that reaches it. | Canonicalize the directory before `starts_with(project)` |
| 6 | `repoview-sources/src/spec.rs:151` | The test `entry_budget_is_twenty_thousand_and_depth_six` only pins the constant. No test runs `roots()` with more than 20,000 entries (grep finds no `20_000` in `tests/sources.rs`), so the mutant `let mut budget = usize::MAX;` at :58 would stay green. I checked this by inspection and did not run the mutant. | CONFIRMED / note | the suite | Once finding 3 is fixed, test the bound through `read_all` |
| 7 | `repoview/src/browser.rs:71` | Untested hypothesis: a snap-confined browser (Ubuntu's default Firefox) cannot read hidden directories under `$HOME`, so the redirect page in `~/.cache/repoview` would fail to load. Jupyter's redirect file reportedly has this problem. | INFEASIBLE / note | Ubuntu with snap Firefox. Not reproducible on this host. The URL is still printed, so the user can still open it by hand. | Put the page under `$XDG_RUNTIME_DIR`, or document the fallback |

**5. Attacked and not broken**
- **Percent-decoding.** I sent 126 traversal URIs to the debug rust-embed source with a symlinked secret: `..`, `%2e%2e`, `.%2e`, `%252e%252e`, overlong `%c0%ae`, separators `/ %2f %5c \ %255c`, and suffixes `%00` and `%00.html`. None leaked. 118 returned 404 and 8 returned the SPA fallback for literal `%2e`/`%5c` names.
- **Process groups.** A setsid'd grandchild holding stdout, and a direct child that calls `setsid -w`, both made `run` return at 1.00 s. Every exit path reaps the child, and the opener is reaped on its own thread. A grandchild that leaves the group with setsid survives the kill; that is outside repoview's control.
- **Redirect directory and file.** A symlinked `repoview` directory is refused, the file uses `create_new` with mode 0600, `/tmp` is refused, and a relative `XDG_CACHE_HOME` falls back to `HOME`. If repoview panics, unwinding drops the page.
- **Pass-1 finding 9** (`project_root` missing from the wire `Section`) is unchanged at `lib.rs:54`. I did not raise it again.

**6. Paths written outside the worktree**
- All under `~/.cache/repoview-wave-one/server-skeleton/scratch/`: `p2-red-walk.log`, `p2-red-fsmon.log`, `p2-red-sigint.log`, `p2-red-sighup.log`, `p2-red-symlink.log`, `p2-probe-decode.log`, `p2-probe-pgroup.log`, `p2-suite.log`, `p2-clippy.log`, `p2-fmt.log`.
- I created and then deleted `scratch/p2-fsmon`, `scratch/p2-proj` and `scratch/p2-url.txt`. I killed the one orphaned `sleep` my probe left, by its PID.
- The build used `~/.cache/b10x-target/repoview-server-skeleton`.
- My lease `adversary-server-skeleton-pass-2` was acquired and released.

```findings
- file: crates/repoview/src/main.rs
  line: 173
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: graceful shutdown waits without bound on a half-received request and a second SIGINT is swallowed, so Ctrl-C no longer stops repoview open
- file: crates/repoview-sources/src/vcs.rs
  line: 58
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "with the user's core.fsmonitor=true, git status starts fsmonitor--daemon, which writes .git/fsmonitor--daemon.ipc and .git/fsmonitor--daemon/ inside the project"
- file: crates/repoview-sources/src/spec.rs
  line: 58
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the depth-first walk outside Git spends its 20,000-entry budget on a large sibling and reports spec Absent for a marker one level below the root
- file: crates/repoview/src/main.rs
  line: 179
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: shutdown_signal ignores SIGHUP, so closing the terminal leaves the token-bearing redirect page in the cache directory
- file: crates/repoview/src/browser.rs
  line: 26
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the inside-the-project check compares uncanonicalized paths, so a cache directory symlinked into the project receives the redirect page
- file: crates/repoview-sources/src/spec.rs
  line: 151
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the entry-budget test pins the constant only; no test drives roots() past 20,000 entries, so dropping the budget at spec.rs:58 stays green
- file: crates/repoview/src/browser.rs
  line: 71
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: untested hypothesis that snap-confined browsers cannot read the redirect page under the hidden ~/.cache directory
```
