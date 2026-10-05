---
format: aep.planning-md/3
id: review-result:adversary-quality-codegate-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:quality-codegate
relations:
- reviews: story:quality-codegate
revision: 1
---
unit: story:quality-codegate, uncommitted working tree on impl/quality-codegate at 03c89a8 (~/.local/state/worktree/trees/b10x/repoview/impl-quality-codegate)
verdict: NEEDS-CHANGE
cases: executed 150→153, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path, the assigned scratch dir ~/.cache/repoview-wave-two/quality-codegate/scratch (listed in part 6)
needs-coordinator: none

**1. Diff.** Unchanged from the implementor's (17 files, +946 / −2117). I added three untracked test files: `crates/repoview-sources/tests/adversary_quality_codegate.rs`, `crates/repoview/tests/adversary_quality_codegate.rs`, `crates/repoview/tests/adversary_quality_codegate_color.rs`.

**2. Cases** (each red when run alone first; output in scratch `adv-red-*.log`)

| Case | Asserts | Red output |
|---|---|---|
| `after_an_in_place_upgrade_the_api_and_the_snapshot_still_agree` | after codegate goes from 0.3.0 to 0.4.0 while the server runs, `/api/quality` and the snapshot's `quality` section name the same path and version | `left: (".../codegate", "0.3.0")  right: (".../codegate", "0.4.0")` |
| `a_user_with_clicolor_force_still_gets_evaluate_and_the_story_reason` | with `CLICOLOR_FORCE=1`, `commands` is `["evaluate"]` and `reason` is the story text | `left: (Array [], "codegate 0.3.0 offers no commands; it has no source assessment yet")` |
| `one_go_codegate_reached_through_a_symlinked_directory_is_probed_and_named_once` | `PATH=usr-bin:bin` where `bin -> usr-bin` gives `skipped` one entry and one `--version` call | `left: ([".../usr-bin/codegate", ".../bin/codegate"], 2)  right: ([".../usr-bin/codegate"], 1)` |

**3. Suite:** `cargo test … --no-fail-fast` `error: 3 targets failed`, EXIT=101: 150 passed, 3 failed (the cases above); fmt and clippy clean; `pnpm check` `Tests  198 passed (198)`, EXIT=0.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| crates/repoview/src/api/quality.rs:414 | NEEDS-CHANGE | introduced | The probe is cached for the whole server run; the snapshot re-locates per request; after an upgrade Quality says 0.3.0 and Overview 0.4.0 (acceptance 4). A removed binary keeps being reported as found | Installing a newer codegate while `repoview open` runs |
| crates/repoview/src/api/quality.rs:325 | NEEDS-CHANGE | introduced | Children inherit the environment; clap colours `--help` under `CLICOLOR_FORCE`; `parse_commands` (:47) never matches `Commands:`; reproduced with the real codegate 0.3.0 through the real repoview binary (`probe-color.out`) | Any user or launcher setting `CLICOLOR_FORCE`; not set on this machine. Fix: `NO_COLOR=1` on the child, or strip ANSI before parsing |
| crates/repoview-sources/src/quality.rs:99 | CONFIRMED | introduced | Dedupe compares the joined path, not the file; one file probed and listed twice | Merged-/usr systems (`/bin -> usr/bin`) with both on PATH and a codegate in `/usr/bin` |

Judgement, no case: any program on PATH printing `codegate <semver>` is accepted (as acceptance 1 specifies); `reason()` would still say "no source assessment" for a future `assess` command (story does not specify).

**5. Attacked and not broken:** Go binary through a logging wrapper received only `--version` (twice: API and snapshot); a hanging `--version` is skipped after 10 s with the group killed; `codegate 1.0.0-rc.1+build` accepted; relative/empty PATH entries, directory, non-executable and dangling symlink handled; the 5 rewritten adversary tests keep their intent (checked by reading, mutants not run); no score/rating/finding rendered (`assessment` never bound; `strayScores` covers stray fields); snapshot locates once per read.

**6. Paths written outside the worktree:** ~/.cache/repoview-wave-two/quality-codegate/scratch/{adv-red-api.log,adv-red-color.log,adv-red-sources.log,adv-suite.log,adv-web.log,probe.sh,gowrap/,hang/,rc/,probe-go-rust.*,probe-hang-rc.*,probe-color.*}; build in the assigned dir.

```findings
- file: crates/repoview/src/api/quality.rs
  line: 414
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the probe is cached per server run while the snapshot re-locates per request, so after an in-place codegate upgrade the Quality page and the Overview card name different versions"
- file: crates/repoview/src/api/quality.rs
  line: 325
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "codegate --help inherits CLICOLOR_FORCE and prints ANSI-styled headings, so parse_commands finds no Commands section and the page reports no commands and the wrong reason"
- file: crates/repoview-sources/src/quality.rs
  line: 99
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "candidates are deduplicated by joined path, not by file, so one codegate reached through a symlinked PATH directory is probed twice and listed twice in skipped"
```
