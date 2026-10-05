---
format: aep.planning-md/3
id: review-result:adversary-quality-codegate-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:quality-codegate
relations:
- reviews: story:quality-codegate
revision: 1
---
```
unit: story:quality-codegate, uncommitted working tree on impl/quality-codegate at 03c89a8 (~/.local/state/worktree/trees/b10x/repoview/impl-quality-codegate)
verdict: CONFIRMED
cases: executed 156→157, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none beyond the assigned scratch dir and build dir (part 6)
needs-coordinator: none
```

The round-1 corrections hold. One real defect, small. The three pass-1 cases are untouched and green.

**1. Diff.** The implementor's 17 files, +1080 / −2114; I changed none. I added `crates/repoview/tests/adversary_quality_codegate_pass2.rs` (rustfmt-clean).

**2. Case** `a_codegate_replaced_while_its_help_runs_is_probed_again_on_the_next_request`: during the server's `--help`, the 0.3.0 stub moves a rebuilt 0.3.0 (same version, adds `assess`) into its own place; the second request should list `["evaluate","assess"]`.
```
panicked at crates/repoview/tests/adversary_quality_codegate_pass2.rs:118:5:
assertion `left == right` failed: the codegate on PATH now offers assess; the answer still lists the replaced binary's commands
  left: Array [String("evaluate")]
 right: Array [String("evaluate"), String("assess")]
```

**3. Gates:** `cargo test … --no-fail-fast` `error: 1 target failed:`, EXIT=101, 156 passed, 1 failed; `pnpm check` `Tests  198 passed (198)`, EXIT=0.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| crates/repoview/src/api/quality.rs:163 | CONFIRMED | introduced | `read_offer` takes `Identity::of` after `--help`; a binary that changes during `--help` caches the old commands under the new identity for the rest of the run. Fix: identity before `--help` | An install landing during the few ms of a real `--help` |
| crates/repoview/src/api/quality.rs:120 | INFEASIBLE | introduced | a same-version, same-size replacement with the old mtime (new inode) keeps the old commands: `Identity` has no inode or ctime; a wrapper that execs a rebuilt binary has the same gap | `cp -p`, `rsync -t` or a shim with a same-version rebuild; nothing found |

Note: `quality.rs:389` sets `CLICOLOR_FORCE=0`, which forces colour in clap (`CLICOLOR_FORCE=0 codegate --help | cat -v` prints `^[[1m^[[4mCommands:`); output stays plain only because `NO_COLOR=1` overrides it and `strip_ansi` backstops.

**5. Attacked, not broken:** call-log changes keep single-start (`adversary_quality.rs` exact `["--version","--help","--version"]`; counts `(3,1)`, `(2,1)`); four concurrent requests after an upgrade give one `--help` and consistent answers; version change between `--version` and `--help` self-heals; ANSI stripping of OSC 8 (ST and BEL), 256/truecolor SGR, CRLF, lone ESC, unterminated CSI; dev+inode dedupe for bind mounts and hard links; cost per request is each candidate's `--version`, 10 s timeout for a hanging one, the page does not poll.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/quality-codegate/scratch/{p2-red-identity.log,p2-suite.log,p2-web.log}`; a temporary probe test was created and deleted.

```findings
- file: crates/repoview/src/api/quality.rs
  line: 163
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Identity::of is taken after --help, so a codegate replaced while --help runs leaves the old commands cached under the new binary's identity for the rest of the server run"
- file: crates/repoview/src/api/quality.rs
  line: 120
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "Identity has no inode or ctime, so a same-version, same-size replacement keeping the old mtime, or a wrapper that execs a rebuilt binary, is never probed again"
```
