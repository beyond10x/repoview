---
format: aep.planning-md/3
id: review-result:adversary-repository-page-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:repository-page
relations:
- reviews: story:repository-page
revision: 1
---
unit: story:repository-page, uncommitted working tree ~/.local/state/worktree/trees/b10x/repoview/impl-repository-page (base 59771f6)
verdict: NEEDS-CHANGE
cases: executed 105→108 Rust, 150→150 web; red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 paths under ~/.cache/repoview-wave-two/repository-page/scratch/adv, plus the assigned build dir
needs-coordinator: the tasks fix needs either a YAML dependency (`Cargo.toml` is not the unit's) or a decision to drop the `task` call

**1. Diff stat.** `git --no-pager diff --stat` shows only the unit's two tracked files (`api/repository.rs`, `RepositoryPage.vue`). The one path I added is the untracked test file `crates/repoview/tests/adversary_repository_page.rs`. No implementation file was touched.

**2. Cases** (each red when run alone with `cargo test -p repoview --locked --test adversary_repository_page`):

| case | asserts | now |
|---|---|---|
| `tasks_listing_executes_no_taskfile_shell_command` | a GET of `/api/tasks` does not run the Taskfile's `vars: PROBE: sh: touch executed-by-listing` command | red, `:96` "GET /api/tasks ran the Taskfile's `sh:` variable command in the project" |
| `vcs_author_with_a_unit_separator_keeps_its_date_and_subject` | an author name containing U+001F comes back whole | red, `:126` `{"author":"Eve","date":"Forged",…,"subject":"2026-10-05T16:01:04+02:00\u001fthe subject"}` |
| `doc_symlinked_into_dot_git_does_not_serve_remote_credentials` | `README.md -> .git/config` does not hand out `x-access-token:abc@` | red, `:155`; the body was the full `.git/config` with the token |

The tasks case was green on its first two runs, for fixture reasons: the test `PATH` had no `touch`, and without `dotenv:` in the Taskfile, task 3.53.1 does not evaluate `vars: sh:` when listing. I added `touch` to the `PATH` and the line `dotenv: ['.env']`; after that it is red. The case returns early if `task` is not on `PATH`.

**3. Suite** (run after the cases existed): `cargo test -p repoview -p repoview-sources --locked --no-fail-fast` EXIT=101, 105 passed, 3 failed (the cases above). `cargo clippy -p repoview --all-targets --locked -- -D warnings` exit 0. `pnpm check` "Test Files 19 passed (19) / Tests 150 passed (150)", EXIT=0.

**4. Findings** (all against the working tree above)

| # | file:line | verdict / origin | what reaches it | fix to name |
|---|---|---|---|---|
| 1 | `crates/repoview/src/api/repository.rs:441` | NEEDS-CHANGE / introduced | `RepositoryPage.vue:27` calls `loadTasks()` when the page opens. A cloned repo whose `Taskfile.yml` has `dotenv:` and an `sh:` var gets that command run by the server. The `--no-status` doc comment (`:428`) suggests listing executes nothing, which is wrong. | Stop running `task` to list tasks: read `Taskfile.yml` in Rust (needs a YAML crate) or remove the route |
| 2 | `crates/repoview/src/api/repository.rs:183` | CONFIRMED / introduced | Only deliberately crafted history: Git accepts U+001F in `user.name`, and a clone keeps it | `--format=%H%x00%aI%x00%an%x00%s` without `-z`, records split on `\n` |
| 3 | `crates/repoview/src/api/repository.rs:382` | CONFIRMED / introduced | A committed symlink `README.md -> .git/config` survives a clone; the token-holder then sees remote credentials unredacted | Refuse a resolved document path inside the git dir (`rev-parse --absolute-git-dir`), or refuse symlinks |

**5. Attacked, held:** porcelain v2 status (renames, newline/tab/space paths, `u` field count, untracked names starting with `# `, non-UTF-8 lossy); nested annotated tags peel; `column.ui=always` ignored with `--format`; worktree `-z` parsing; remote URL redaction (`git+https://u:p@`, `%40`, `p@ss@host`, IPv6, `persistent-https::`, `branch.<b>.remote` set to a token URL); `.git/index` unchanged with split index and untracked cache; no `v-html`/`href` outside MarkdownView; FIFO or directory named README.md shows absent; file growing past the limit caught by `take()`.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/repository-page/scratch/adv/{taskprobe,g,u,t2,sl,slc}`, `…/adv/{suite.log,suite-nff.log,web.log}`, the assigned build dir; lease `adversary-repository-page` taken and released.

```findings
- file: crates/repoview/src/api/repository.rs
  line: 441
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "GET /api/tasks runs `task --list-all --json --no-status`, which evaluates the Taskfile's `vars: sh:` commands when it declares dotenv, so opening the page runs a cloned repository's shell code"
- file: crates/repoview/src/api/repository.rs
  line: 183
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an author name containing U+001F shifts the git log fields, so author, date and subject come back wrong
- file: crates/repoview/src/api/repository.rs
  line: 382
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a committed README.md symlink to .git/config passes the inside-root check, so /api/docs serves remote URLs with their credentials"
```
