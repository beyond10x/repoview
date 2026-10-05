---
format: aep.planning-md/3
id: review-result:adversary-static-export-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:static-export
relations:
- reviews: story:static-export
revision: 1
---
unit: story:static-export, uncommitted working tree in `~/.local/state/worktree/trees/b10x/repoview/impl-static-export` (base cfc7afa)
verdict: NEEDS-CHANGE
cases: executed 210→213, red 3
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths, all under the assigned scratch dir (part 6)
needs-coordinator: none

The export lets `--force` delete or overwrite files outside `--out` through a symlink, and tool paths outside `$HOME` reach the export inside error messages. Three new tests are red.

**1. Diff stat.** Only the unit's own 3 tracked files (lib.rs, main.rs, client.ts: 89 insertions, 4 deletions). My only write is the new untracked test file `crates/repoview/tests/export_adversary.rs`.

**2. Cases added** (each run alone, all red):

| Test | Asserts | Red output |
|---|---|---|
| `force_never_removes_a_file_outside_out_through_a_symlinked_directory` | `--out/link -> ../outside`, manifest lists `link/victim.txt`; after `--force` the outside file still exists | `--force removed ~/.cache/claude-tmp/.tmpDv8uyD/outside/victim.txt although it is outside --out … (export returned Ok(…))` `left: None` `right: Some("precious")` |
| `force_never_writes_through_a_symlink_to_a_file_outside_out` | `--out/index.html` is a symlink to an outside file and the manifest does not list it; the outside file keeps its content | `the export overwrote ~/.cache/claude-tmp/.tmph86ReB/outside/victim.txt outside --out …` `left: Some("<!doctype html>…content=\"static\"…")` |
| `no_exported_file_holds_a_tool_path_outside_home_in_a_diagnostic` | `aep` outside project and home, `--version` exits 1 with empty stderr; no exported file contains that path | `the tool_path value ~/.cache/claude-tmp/.tmpaxAJON/opt/tools/bin/aep appears in ["data/snapshot.json"]` `"diagnostic": "…/opt/tools/bin/aep exited with exit status: 1"` |

The removal attack was also run against the real binary: `repoview export --force --out` exited 0 and `outside/victim.txt` was gone.

**3. Suite:** `cargo test … --no-fail-fast` exit 101, passed 210, failed 3 (`export_adversary`); `pnpm check` exit 0, `Tests 537 passed (537)`.

**4. Findings**

| # | file:line | Verdict / origin | Finding | What reaches it |
|---|---|---|---|---|
| F1 | `crates/repoview/src/export.rs:282` | CONFIRMED / introduced | `remove_previous` checks only the last component with `symlink_metadata`; a symlinked directory earlier in the path is followed, so `--force` deletes outside `--out` (acceptance 5) | `--force` on a directory holding a symlinked subdirectory and a manifest naming a file under it (e.g. a committed export regenerated in CI) |
| F2 | `crates/repoview/src/export.rs:319` | CONFIRMED / introduced | `write` uses `fs::write`, which follows a symlink at the target name and overwrites the file it points to | Same shape as F1 |
| F3 | `crates/repoview/src/export.rs:562` | CONFIRMED / introduced | Only the `tool_path` key is shortened; the same path in "exited with" (`process.rs:108`) and "timed out after" (`:81`) messages is written as-is (acceptance 3) | Any tool outside `$HOME` (`/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin/git`) exiting non-zero with empty stderr or timing out |
| F4 | `web/src/api/client.ts:136` | INFEASIBLE / introduced (note) | `.error.json` is consulted only after a 404; a host answering missing files with 200 `index.html` or 403 never shows recorded errors | Host behaviour taken from documentation, not observed |

Suggested: F1/F2 refuse any symlink between `--out` and the target, or write into a fresh directory and rename it into place; F3 replace each snapshot `tool_path` with its file name in every string the scrub touches.

**5. Attacked and could not break:** real export of this worktree (152 routes) has no `/home/`, `/root`, `/Users/` outside quoted artifact text, no token, project root `.`, home `~`; remote credentials removed by `redact_url`; ids, spec roots and doc names with `..`, `.`, empty or absolute segments skipped; non-relative manifest names refused; served offline at `/repoview/demo/`, 8 hash routes in headless Chrome rendered from 80 requests, none to `/api`; assets referenced `./assets/`, no `<base>`; `:` in file names fine on Linux/macOS; `--out` inside the project is not read back.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/static-export/scratch/{site1/,www/,chrome/,http.log,probe-force/,adversary-suite.log,adversary-suite-nff.log,adversary-web.log}`; static server stopped; lease released.

```findings
- file: crates/repoview/src/export.rs
  line: 282
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "--force follows a symlinked directory inside --out and deletes a manifest-listed file outside --out"
- file: crates/repoview/src/export.rs
  line: 319
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "write follows an existing symlink at a target name inside --out and overwrites a file outside --out"
- file: crates/repoview/src/export.rs
  line: 562
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "only the tool_path key is cut to a file name; the same path in exited-with and timed-out messages is exported verbatim"
- file: web/src/api/client.ts
  line: 136
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "error files are consulted only after a 404, so a host answering missing files with 200 index.html or 403 never shows recorded errors; no such host shown in use"
```
