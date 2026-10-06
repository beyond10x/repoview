---
format: aep.planning-md/3
id: review-result:adversary-taskfile-tasks-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:taskfile-tasks
relations:
- reviews: story:taskfile-tasks
revision: 1
---
```
unit: story:taskfile-tasks, branch impl/taskfile-tasks at 0be2ff5 (base cfc7afa) plus my one untracked test file
verdict: NEEDS-CHANGE
cases: executed 219→225, red 6
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 scratch logs, plus the assigned build dir
needs-coordinator: none
```

Verdict: the corrections do not hold. I added 6 cases and all 6 are red. Pass 1's file is unchanged and green.

## 1. What I touched

No tracked file changed; one untracked test file `crates/repoview/tests/adversary_taskfile_tasks_pass_2.rs`. A temporary probe file was created and deleted.

## 2. Cases added (each red the first time, run alone; debug build)

| Case (line) | What it asserts | Red output |
|---|---|---|
| `flow_nesting_hidden_by_quoted_closers_is_refused_quickly` (:95) | 40,000 levels of `["]", ` (280 KB) answer 503 in under 10 s | `40000-deep nesting behind quoted closers took 28.219897642s` |
| `includes_that_list_nothing_are_bounded_too` (:118) | the implementor's fan-out tree without tasks answers in under 10 s | `an include tree that lists nothing took 37.486915438s` |
| `a_large_description_included_many_times_does_not_multiply_the_response` (:145) | a 100 KB `desc` included 1,000 times gives a body under 64× the bytes on disk | `118323 bytes on disk answered 102472954 bytes` (peak 409 MB) |
| `an_aliased_long_scalar_is_refused_like_the_flat_expansion` (:172) | a 32 KB anchor used 4,000 times answers 503 | `131072000 bytes of aliased scalar were built`, `left: 200 right: 503` (peak 136 MB) |
| `taskfiles_after_the_node_budget_is_spent_are_refused_without_parsing` (:196) | 40 includes refused by a spent node budget answer in under 10 s | `40 includes refused by a spent node budget took 22.947399519s` |
| `unclosed_brackets_inside_strings_do_not_add_up_to_nesting` (:228) | 65 `echo -e "\033[32m…\033[0m"` commands list the task | `diagnostic: "Taskfile.yml: nested more than 128 levels deep; not parsed"`, `left: 503 right: 200` |

## 3. Gates

`cargo test … --no-fail-fast` EXIT=101, 219 passed, 6 failed (all in `adversary_taskfile_tasks_pass_2`); `pnpm check` EXIT=0, `Tests  542 passed (542)`; my file rustfmt and clippy clean.

## 4. Findings (all in `crates/repoview/src/api/repository.rs`, at 0be2ff5)

| # | Line | Verdict | Origin | What was measured | What reaches it |
|---|---|---|---|---|---|
| 1 | :933 | NEEDS-CHANGE | introduced | the pre-scan counts a quoted `]` as a close; libyaml goes 40,000 deep: 28 s for 280 KB (pass 1 finding 3 again) | Every Repository page visit; a hostile Taskfile |
| 2 | :779 | NEEDS-CHANGE | introduced | includes that list nothing are never counted: 40 per file, 4 deep, 6 KB, 37 s; grows as width⁴ | Same |
| 3 | :1028 | NEEDS-CHANGE | introduced | a scalar costs 1 node whatever its length: 48 KB built 128 MB; ~89 GB at 1 MiB | Same |
| 4 | :1098 | CONFIRMED | introduced | each of 10,000 entries copies `desc` whole: 118 KB gave 102 MB; 10 GiB at the limits | Same |
| 5 | :736 | CONFIRMED | introduced | files after the node budget is spent are still fully loaded: 127 files took 78 s | Same |
| 6 | :927 | INFEASIBLE | introduced | the bracket count never resets; unclosed `[` in strings add up; a 130-line indented block scalar is refused | none of 134 local Taskfiles (deepest 5, all balanced) |

**5. Attacked and could not break:** pass-1 file 4 passed; replaced case keeps the 10 s bound plus `entries == 10_000` and `truncated`; parse cache handles `big.yml`, `./big.yml`, `.//big.yml`, symlinks, directory includes; nesting limit agrees with serde_yaml_ng at depths 120–130; CRLF, tabs in block scalars, `[` in comments return 200; tags work; 190 merge sites over a 1,000-key anchor refused in 0.6 s; `apply_merge` moves, never clones; a 1 MiB file at the limits takes 0.33–1.1 s.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-three/taskfile-tasks/scratch/{p2-suite.log,p2-web.log,p2-<case>.log ×6}`; the assigned build dir.

```findings
- file: crates/repoview/src/api/repository.rs
  line: 933
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The nesting pre-scan counts a quoted \"]\" as a close, so 40,000 levels of [\"]\", reach libyaml unrefused and take 28 s for 280 KB, the quadratic scan of pass 1 finding 3 again."
- file: crates/repoview/src/api/repository.rs
  line: 779
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The entry cap counts only tasks and refusals, so an include tree that lists nothing (40 wide, 4 deep, 6 KB) walks 2.6 million includes in 37 s, and cost grows as width to the fourth."
- file: crates/repoview/src/api/repository.rs
  line: 1028
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The node budget charges one node per scalar whatever its length, so an anchored 32 KB string aliased 4,000 times builds 128 MB from 48 KB, about 89 GB at the 1 MiB cap."
- file: crates/repoview/src/api/repository.rs
  line: 1098
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Each of the 10,000 permitted entries copies its desc whole, so one 100 KB desc included 1,000 times answers 102 MB from 118 KB on disk, 10 GiB at the limits."
- file: crates/repoview/src/api/repository.rs
  line: 736
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Taskfiles read after the node budget is spent are still loaded in full by libyaml before refusal: 40 node-heavy 1 MiB includes take 23 s and 127 take 78 s in a debug build."
- file: crates/repoview/src/api/repository.rs
  line: 927
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The flow counter never resets, so unclosed brackets in strings such as ANSI colour codes add up across the file and refuse a valid Taskfile as nested over 128 levels; no local Taskfile does this."
```
