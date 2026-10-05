---
format: aep.planning-md/3
id: review-result:adversary-taskfile-tasks-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:taskfile-tasks
relations:
- reviews: story:taskfile-tasks
revision: 1
---
I made the unit fail: 4 cases I added are red. All 4 come from this unit's own diff, and I changed no implementation file.

```
unit: story:taskfile-tasks, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-taskfile-tasks (base cfc7afa plus its uncommitted diff)
verdict: NEEDS-CHANGE
cases: executed 208→212, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 scratch paths, plus the assigned build dir
needs-coordinator: is a hostile repository in repoview's threat model? That sets the severity of findings 1–3.
```

## 1. What I touched

Same 6 files and 486+/23− as handed over; my only addition is the untracked `crates/repoview/tests/adversary_taskfile_tasks.rs`.

## 2. Cases added (all red; each run alone first)

| Case | What it asserts | Red output (abridged) |
|---|---|---|
| `a_flat_alias_expansion_is_refused_like_the_nested_bomb` | A 21 KB Taskfile with one anchored list of 3000 items, referenced 3000 times, answers 503 | `a 3000×3000 alias expansion was parsed in full (41.481428093s)`, `left: 200 right: 503`; peak memory 923,712 KB |
| `one_file_included_many_times_does_not_multiply_without_bound` | One 949 KB file of 80,000 tasks, included 32 times, answers in under 10 s with a body under 64× the bytes on disk | `949409 bytes on disk answered 201084526 bytes in 51.785305834s`; peak memory 4,575,812 KB |
| `deep_flow_nesting_is_refused_quickly` | 40,000-deep `[`…`]` nesting (80 KB) is refused in under 10 s | `40000-deep nesting took 40.521624173s` (20,000 deep took 8.4 s) |
| `a_merge_key_supplies_the_desc_as_task_does` | A `<<: *defaults` key inside a task supplies its `desc` | `left: Null right: "Shared description"` |

Debug-build times. The 64× bound is mine; the 10 s bound copies the unit's own bomb and fan-out tests.

## 3. Suite run

`cargo test … --no-fail-fast` EXIT=101, 208 passed, 4 failed (only `adversary_taskfile_tasks`); `pnpm check` EXIT=0, `Tests 541 passed (541)`; my file rustfmt and clippy clean.

## 4. Findings (all cite `crates/repoview/src/api/repository.rs`)

| # | Line | Verdict | Origin | What was measured | What reaches it |
|---|---|---|---|---|---|
| 1 | :687 | NEEDS-CHANGE | introduced | serde_yaml_ng counts alias uses, not the nodes they copy; a flat expansion passes the guard `an_alias_bomb_fails_quickly` relies on: 21 KB cost 41 s and 0.9 GB | Repository page loads `/api/tasks` on every visit; a hostile Taskfile |
| 2 | :773 | CONFIRMED | introduced | The 128-file budget counts reads, but one file can be read again under another namespace: two files on disk gave 201 MB, 52 s, 4.6 GB | Same |
| 3 | :682 | CONFIRMED | introduced | Only file size is checked before parsing; deep nesting is refused after libyaml scans the whole file (80 KB ≈ 40 s) | Same |
| 4 | :821 | INFEASIBLE | introduced | `<<` merge keys are never applied, so a merged `desc` shows null; go-task applies them (from recollection, not fetched) | No local Taskfile uses `<<:` |

Suggested: node-counting visitor with a limit; cap total tasks and refused entries per response, or read each file once; byte-level nesting-depth pre-scan refusing over 128; `apply_merge()` after the expansion guard.

## 5. Attacked and could not break

Leaving the project (absolute, `~`, `..`, remote, templated paths, symlinks out or into `.git`, a worktree's `.git` file); default names match go-task's 8 in order (from memory); `!!binary`, custom and tagged keys give 200, duplicate keys 503, no panic; middle-folder swap race untestable deterministically; hardlinks cannot be committed; `TasksList.vue` uses only `{{ }}`, `MarkdownView` is the only `v-html`.

## 6. Paths written outside the worktree

`~/.cache/repoview-wave-three/taskfile-tasks/scratch/{deep.rs,suite.log,suite-nff.log,web.log}`, the assigned build dir; lease taken and released.

```findings
- file: crates/repoview/src/api/repository.rs
  line: 687
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A flat alias expansion (one anchored 3000-item list referenced 3000 times, 21 KB) bypasses serde_yaml_ng's jump-count limit and is fully expanded: 41 s and 0.9 GB, scaling with N×M up to the 1 MiB cap."
- file: crates/repoview/src/api/repository.rs
  line: 773
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Re-including one 949 KB file of tasks 32 times yields a 201 MB response in 52 s with 4.6 GB peak memory, because the 128-file budget does not bound tasks or repeated reads of the same file."
- file: crates/repoview/src/api/repository.rs
  line: 682
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Deep flow nesting is refused only after libyaml scans the whole file, which grows faster than quadratic in depth (20k deep 8.4 s, 40k deep 40.5 s), so a sub-1 MiB Taskfile holds the request thread for hours."
- file: crates/repoview/src/api/repository.rs
  line: 821
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "YAML merge keys are not applied, so a desc supplied through a merge key reads as null where go-task shows it; no local Taskfile was found using merge keys."
```
