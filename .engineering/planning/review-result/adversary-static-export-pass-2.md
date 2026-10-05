---
format: aep.planning-md/3
id: review-result:adversary-static-export-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:static-export
relations:
- reviews: story:static-export
revision: 1
---
```
unit: story:static-export, uncommitted working tree ~/.local/state/worktree/trees/b10x/repoview/impl-static-export (base cfc7afa, after correction round 1)
verdict: CONFIRMED
cases: executed 216→220, red 4
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths, all in the assigned scratch dir (part 6)
needs-coordinator: none
```

The round-1 corrections hold and none of the pass-1 findings is back. Pass 2 found two new defects and one gap in the correction, each with a red test: a sibling path beside the project root (`git worktree add ../proj.next`) is rewritten to `..next`; a tool path inside a longer path is spliced (`/usr/bin/git` → `/usrgit`); `--out site/` with a trailing slash follows a symlink that `--out site` refuses. None leaks a token or a home path.

**1. Diff stat:** `crates/repoview/src/lib.rs | 1`, `main.rs | 66`, `web/src/api/client.ts | 101` (implementor's). My only write is the untracked `crates/repoview/tests/export_adversary_pass2.rs`.

**Corrections verified:** `export_adversary.rs` 3 tests and messages match the pass-1 record, 3 passed. `client.static.test.ts` rewrite: the 4 round-0 red tests have successors; "a 404 with no error file is a 404" (the removed fallback) is replaced by "a route listed 2xx or not listed is never read from an error file"; web 537 → 541. Symlink refusal, temp-file-and-rename writes, tool path to file name, `export.json` consulted whatever the host answers: hold except as below.

**2. Cases added** (each red alone first):

| Test | Asserts | Red output (verbatim) |
|---|---|---|
| `a_sibling_worktree_whose_name_extends_the_roots_keeps_its_own_name` | after `git worktree add ../proj.next`, the exported worktree path still ends in `proj.next` | `the sibling worktree ~/proj.next is exported as [".", "..next"]` |
| `a_root_or_home_spelling_followed_by_a_dot_is_not_a_whole_component` | `Scrub` leaves `/srv/p.git` alone when the root is `/srv/p` | `left: "remote ..git"` `right: "remote /srv/p.git"` |
| `a_tool_path_inside_a_longer_path_is_not_spliced` | tool `/bin/git`: `/usr/bin/git` not spliced, `/bin/git` does not survive | `a different path was spliced: "hooks run /usrgit and git"` |
| `an_out_directory_symlink_spelt_with_a_trailing_slash_is_refused` | `--out <link>/` refused like `--out <link>` | `--out ~/.cache/claude-tmp/.tmpslNaFg/site/ (a symlink to …/real) was not refused: Ok(Summary { … }); …/real now holds ["data", "index.html"]` |

**3. Suite:** `cargo test … --no-fail-fast` exit 101, 216 passed, 4 failed (only `export_adversary_pass2`); `pnpm check` exit 0, `Tests  541 passed (541)`; clippy clean; rustfmt clean on the new file.

**4. Findings**

| # | file:line | Verdict / origin | Finding | What reaches it |
|---|---|---|---|---|
| F1 | `crates/repoview/src/export.rs:747` | CONFIRMED / introduced | `replace_component` treats `.` after the root or home path as the end of a component (`<root>.next` → `..next`, `<home>.old` → `~.old`); left boundary not checked | `git worktree add ../proj.<x>` listed on the vcs page, or a local bare remote `../proj.git` |
| F2 | `crates/repoview/src/export.rs:700` | CONFIRMED / introduced | tool rewrite is an unbounded `str::replace`; `/bin/git` inside `/usr/bin/git` produces `/usrgit` | `PATH` with `/bin` before `/usr/bin` plus exported text naming `/usr/bin/<tool>`; no real example found |
| F3 | `crates/repoview/src/export.rs:285` | CONFIRMED / introduced | `symlink_metadata("link/")` follows the link, so the refusal misses `--out site/` | Typing `--out site/` where `site` is a symlink |
| J1 | `crates/repoview/src/export.rs:421-450` | INFEASIBLE / introduced | check-then-open/rename by path; a parent swapped for a symlink in between escapes `--out` (fix: `openat` handles) | Only a concurrent writer into `--out` |
| J2 | `web/src/api/client.ts:161` | INFEASIBLE / introduced | cached `export.json` outlives a re-export while the page is open | Re-export with `--force` while a browser is open; reload fixes it |

**5. Attacked and could not break:** pass-1 symlink cases green; temp files removed on error (only Ctrl-C leaves a dotfile); rename stays in the target's directory; relative or Windows tool paths unreachable (`find_tool` keeps absolute dirs); replacement keys are full paths, so a plain word like `git` is untouched; client route keys match manifest paths; a host answering 403, 500 or 200 HTML still reaches the recorded error.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/static-export/scratch/{adversary2-red.log,adversary2-suite.log,adversary2-web.log}`, the assigned build dir; lease released.

```findings
- file: crates/repoview/src/export.rs
  line: 747
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "replace_component ends a component at '.', so a sibling such as <root>.next is exported as '..next' and <home>.old as '~.old'"
- file: crates/repoview/src/export.rs
  line: 700
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the tool rewrite is an unbounded str::replace, so tool path /bin/git turns /usr/bin/git into /usrgit"
- file: crates/repoview/src/export.rs
  line: 285
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "--out <symlink>/ with a trailing slash passes the symlink refusal because symlink_metadata follows the link"
- file: crates/repoview/src/export.rs
  line: 421
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "write checks for symlinks then opens and renames by path, so a parent swapped for a symlink in between escapes --out; needs a concurrent writer into --out"
- file: web/src/api/client.ts
  line: 161
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "the cached export.json routes outlive a re-export while the page is open, so a route now erroring answers HTTP 404 until reload"
```
