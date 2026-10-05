---
format: aep.planning-md/3
id: review-result:adversary-spec-pages-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:spec-pages
relations:
- reviews: story:spec-pages
revision: 1
---
I made the unit red with 2 cases. Both come from the same place: the server runs `ess --version` before it checks whether the requested root is one it detected. That breaks acceptance 1 ("else 404 and no process starts"). The SPA never sends such a request; only a direct API call with a valid token does.

```
unit: story:spec-pages, uncommitted working tree on base 59771f6 (~/.local/state/worktree/trees/b10x/repoview/impl-spec-pages)
verdict: NEEDS-CHANGE
cases: executed 93→95 Rust (web 165→165), red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (scratch, listed in part 6)
needs-coordinator: whether acceptance 1 should read "no `ess specify` starts", or the code should change (with `git` on PATH, root detection itself runs `git ls-files`)
```

**1. Diff.** `git diff --stat` shows only the implementor's 3 tracked files. My one addition is the untracked test file `crates/repoview/tests/api_spec_adversary.rs`. I changed no implementation file.

**2. Cases added**, both in `api_spec_adversary.rs`, both red when first run on their own:

| case | asserts | red output |
|---|---|---|
| `an_undetected_root_starts_no_process_at_all` | `../x/ir`, `..%2Fx/graph` and `nope/mermaid` each return 404, and the stub `ess` logs zero calls (it logs `--version` too) | `left: ["--version", "--version", "--version"]  right: []` at :132 |
| `an_undetected_root_is_404_even_when_ess_is_missing` | an undetected root returns 404 when no `ess` is on PATH | `/api/spec/roots/../x/ir: {"exit":null,"stderr":"ess not found on PATH","tool":"ess"}  left: 503  right: 404` at :142 |

**3. Suite runs:** `cargo test -p repoview -p repoview-sources --locked --no-fail-fast` 93 passed, 2 failed (`api_spec_adversary`), exit 101; clippy `-D warnings` exit 0; `cd web && pnpm check` `Tests 165 passed (165)`, exit 0.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `crates/repoview/src/api/spec.rs:214` | NEEDS-CHANGE | introduced | `detected()` calls `source.read`, which runs `ess --version` before the root check at :180; the unit's own test only proves no `specify` call ran | Any token-holding GET to an undetected `{root}`; the URL never reaches the process. Fix: root check before the version probe, using the `roots` walk in `repoview-sources/src/spec.rs`, or reword the acceptance |
| `crates/repoview/src/api/spec.rs:228` | CONFIRMED | introduced | With `ess` missing, an undetected root returns 503 instead of 404 | Direct API calls only |

**5. Attacked, could not break:** roots starting with `-` stay one argument (`--path=-rf`); a sleeping stub times out at 10 s and its child is killed; 30 MB stdout + 100 KB stderr pass through; case, trailing slash, `//`, view `IR` → 404; roots needing encoding (`my spec`, `a#b`, `100%`, `q?x`, `ünï/sub`, `docs/ir`, `x/~`, `-rf`, `a%2Fb`) work end to end; every lifecycle diagram from the fixtures and hostile labels (`end`, `note`, `[*]`, `click … call`, `classDef`, `-->`) parse in Mermaid; ess 0.52 rejects `%`, `<` and lowercase in identifiers. The comment in `web/src/api/spec.ts` says the pages "guard each one"; they don't (`lifecycle: null`, missing `outcomes` or `identity` would throw); no current ess emits those, so no case.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/spec-pages/scratch/{adv-suite.log,adv-suite-nff.log,adv-web.log,essroots.txt,probe1/}`; build dir assigned; lease `adversary-spec-pages` taken and released.

```findings
- file: crates/repoview/src/api/spec.rs
  line: 214
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "an undetected {root} still starts `ess --version` (via source.read) before the 404, against acceptance 1's \"no process starts\"; the unit's test only checks for `specify` calls"
- file: crates/repoview/src/api/spec.rs
  line: 228
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "with ess missing, an undetected {root} answers 503 instead of 404, because roots are only known after the tool probe"
```
