---
format: aep.planning-md/3
id: review-result:adversary-server-skeleton-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:server-skeleton
relations:
- reviews: story:server-skeleton
revision: 1
---
I broke story:server-skeleton: 6 new test cases fail, and 2 of the defects have to change before the unit merges. I edited no implementation file and created no commits.

```
unit: story:server-skeleton, uncommitted working tree on base d411e1c (worktree impl-server-skeleton)
verdict: NEEDS-CHANGE
cases: executed 47→53, red 6
origin: introduced 9 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 (scratch logs, listed in part 6)
needs-coordinator: none
```

**1. What I touched.** `git --no-pager diff --stat` is empty because the whole unit is untracked. `git status --short --untracked-files=all | grep -i adversary` shows the three new test files below. There is no non-test path.

**2. Cases added.** All were red on their first run, each run on its own before the suite.

| file | test | asserts | red output (verbatim) |
|---|---|---|---|
| `crates/repoview-sources/tests/adversary.rs` | `snapshot_does_not_rewrite_the_git_index` | `.git/index` is byte-for-byte unchanged after a snapshot of a repo whose index stat data is stale | `the snapshot rewrote .git/index inside the project directory` |
| same file | `run_returns_within_the_timeout_when_a_grandchild_keeps_stdout_open` | `run` with a 1 s timeout returns in under 3 s | `run with a 1 s timeout took 6.001540483s (Success { stdout: "v\n" })` |
| `crates/repoview/tests/adversary_router.rs` | `api_with_trailing_slash_is_404_not_the_spa` | `GET /api/` with a valid token returns 404 | `left: 200 right: 404`, body `<!doctype html><title>spa</title>` |
| same file | `backslash_dot_dot_does_not_escape_the_asset_folder` | a static request never serves a file outside the asset folder | `200 OK served a file outside the asset folder for /..\..\…\<home>\.cache\claude-tmp\.tmpNIfYvr\link` |
| `crates/repoview/tests/adversary_cli.rs` | `root_pointing_at_a_regular_file_fails` | `--root <file>` exits non-zero | `exit 0 with --root <file>; stdout: {` followed by a full snapshot |
| same file | `browser_launch_does_not_run_an_opener_from_the_project_directory` | with `PATH=":<bin>"`, an `./xdg-open` in the project directory does not run | `repoview ran ./xdg-open from the project directory` |

Supporting probes, all in scratch:
- On a plain repo, `git --no-optional-locks status` left the index sha1 unchanged; plain `git status` changed it (`5ff3d59…` → `289a9fd…`).
- Over a real socket, the server answered 200, not 400, to a raw backslash path, so hyper accepts backslash paths and the traversal reaches the handler over HTTP.
- Over the same socket, `GET /api/?token=…` returned 200.

**3. Suite run** (after the cases existed):
- `cargo test -p repoview -p repoview-sources --locked --no-fail-fast` exited 101: 6 failed (all mine), 47 passed.
- The 47 comes from a second run with the three adversary test binaries deselected: 3+0+1+15+9+0+19 = 47.
- `cargo clippy -p repoview -p repoview-sources --all-targets --locked -- -D warnings` exited 0.
- `cargo fmt -p repoview -p repoview-sources -- --check` exited 0.

**4. Findings** (they cover the uncommitted tree on d411e1c)

| # | file:line | finding | verdict / severity | what reaches it | fix (named, not applied) |
|---|---|---|---|---|---|
| 1 | `repoview-sources/src/vcs.rs:58` | `git status --porcelain` rewrites `.git/index` and takes `index.lock`. That breaks AGENTS.md "repoview writes nothing inside the project directory", and a concurrent `git commit` by the user can fail on the lock. | NEEDS-CHANGE / blocker | every `/api/snapshot` poll and every `repoview snapshot` in a repo with stale stat data (after a checkout, or after an editor touches a file) | `git --no-optional-locks status`, or `GIT_OPTIONAL_LOCKS=0` for every git child |
| 2 | `repoview/src/server.rs:116` | the `..` check splits only on `/`. rust-embed's debug `get` turns `\` into `/` and lets the result through when the final path component is a symlink, so `/..\..\…\target` reads files outside `web/dist`. | NEEDS-CHANGE / warning | debug binary (`cargo run`) with `web/dist` built; static paths need no token; the attacker is another local user or process (browsers normalise `\`, so a web page cannot do this). Release builds use the embedded map and are not affected. | refuse any path containing `\`, or `.`/`..` segments after normalising `\` |
| 3 | `repoview/src/server.rs:50` | `/api/` with a valid token falls through to the SPA (200 HTML). The contract says API paths never fall back. | CONFIRMED / note | any client that requests `/api/` | answer 404 for every path starting `/api` that no route matched |
| 4 | `repoview/src/project.rs:27` | `--root <regular file>` exits 0 and prints a project named after the file | CONFIRMED / warning | a typo such as `--root README.md` | check `is_dir()` after canonicalize |
| 5 | `repoview/src/main.rs:176` | the browser launcher looks the opener up through `execvp`, which reads an empty `PATH` entry as the working directory, so `./xdg-open` in the project runs. The sources already skip relative entries. | INFEASIBLE / warning | needs a user `PATH` with an empty entry (for example `PATH=$UNSET:$PATH`); not shown to exist here | resolve the opener with the absolute-only `find_tool` |
| 6 | `repoview-sources/src/process.rs:74` | the 10 s timeout does not cover reading the output: `stdout.join()` blocks until every holder of the pipe exits | INFEASIBLE / warning | a tool or wrapper that leaves a background child holding stdout; the test builds this case, and no real tool was found that does it | wait on the reader threads only for the time left in the budget, or kill the process group |
| 7 | `repoview/src/main.rs:176` | the token-bearing URL is passed as an argument to `xdg-open` or the browser, so any local user can read it from `/proc/<pid>/cmdline` | CONFIRMED / note | default `repoview open` without `--no-browser`; the design chose this | open through a redirect file instead, as Jupyter does |
| 8 | `repoview-sources/src/spec.rs:99` | outside Git, every snapshot walks the whole tree with no depth or time limit (for example when run from `$HOME`) | CONFIRMED / note | the discovery case "outside any Git repository" in acceptance 2; the cost was not measured, so this is a hypothesis | add a depth limit, a file-count limit or a time budget |
| 9 | `repoview-sources/src/lib.rs:54` | the ESS `repoview.project.Source` declares `project_root` (`ess/domains/project.yaml:49`), and the wire omits it. The story's contract example omits it too. | CONFIRMED / note | anything generated from `ess/` later | make the ESS and the story contract agree |

**5. Attacked and not broken**
- Token compare uses `subtle` `ct_eq`. An empty or uppercase token is refused. A wrong header beats a right query token. The token does not appear in error bodies; it reaches stderr only on browser-launch failure, which is intended.
- Host check refuses: a missing Host, a wrong port, `localhost.evil.example`, a trailing dot, and any IPv6 host (the server binds IPv4 only). Uppercase hosts are accepted.
- `%2e%2e` and `%2F` are not decoded, so they reach no file. `//api/snapshot` falls through to static with no data. `/apix` is static. `/api//x` needs the token.
- Non-UTF-8 tool output becomes lossy text. A hung direct child is killed. `find_tool` skips relative `PATH` entries. A nonexistent `--root` fails cleanly with exit 1.

**6. Paths written outside the worktree**
- `~/.cache/repoview-wave-one/server-skeleton/scratch/adv-red-repoview.log`
- `~/.cache/repoview-wave-one/server-skeleton/scratch/suite-after.log`
- I also created probe directories (`scratch/idxprobe`, `scratch/proj`, `scratch/open.out`) and deleted them.
- The build used the unit's own `~/.cache/b10x-target/repoview-server-skeleton`.
- My worktree session lease (`adversary-server-skeleton`) was acquired and released.

```findings
- file: crates/repoview-sources/src/vcs.rs
  line: 58
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "git status without --no-optional-locks rewrites .git/index on every snapshot, breaking the read-only rule and racing the user's index.lock"
- file: crates/repoview/src/server.rs
  line: 116
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the traversal guard splits only on '/', so a backslash path reaches rust-embed's debug get and reads symlinked files outside web/dist without a token
- file: crates/repoview/src/server.rs
  line: 50
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: GET /api/ with a valid token returns the SPA with 200 instead of 404
- file: crates/repoview/src/project.rs
  line: 27
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: --root pointing at a regular file exits 0 and snapshots a project named after the file
- file: crates/repoview/src/main.rs
  line: 176
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the browser opener is resolved by execvp, so an empty PATH entry runs ./xdg-open from the project directory
- file: crates/repoview-sources/src/process.rs
  line: 74
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the subprocess timeout does not bound the pipe joins, so a grandchild holding stdout stalls run past its timeout
- file: crates/repoview/src/main.rs
  line: 176
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the run token is passed in the browser launcher's argv and is readable by any local user through /proc
- file: crates/repoview-sources/src/spec.rs
  line: 99
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: outside Git every snapshot walks the entire tree with no depth or time bound
- file: crates/repoview-sources/src/lib.rs
  line: 54
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the wire Section omits project_root, which ess/domains/project.yaml declares on repoview.project.Source
```
