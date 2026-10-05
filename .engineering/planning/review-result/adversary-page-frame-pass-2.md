---
format: aep.planning-md/3
id: review-result:adversary-page-frame-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:page-frame
relations:
- reviews: story:page-frame
revision: 1
---
I found no defect in the corrected code. Both pass-1 fixes hold. I added no test cases. The one finding left is a contract gap for a spec root with the id `.`, and nothing I could find reaches it.

```
unit: story:page-frame, working tree ~/.local/state/worktree/trees/b10x/repoview/impl-page-frame (59771f6 + round-1 corrections, uncommitted)
verdict: INFEASIBLE (1 note; no failing case)
cases: executed 124→124, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3
needs-coordinator: route the `.` root finding to story:spec-pages, or close it
```

**1. Diff stat.** I changed no files. `git diff --stat` lists only the implementer's 4 corrections: `client.ts`, `client.test.ts`, `MermaidView.vue`, `MermaidView.test.ts` (+86/−7). The untracked files are the same 4 pass-1 adversary tests.

**Corrections check**
- **No weakened assertion:** `git diff 59771f6 -- web/src` only adds cases. It also tightens the `initialize` assertion with `suppressErrorRendering: true`.
- **Pass-1 adversary files:** last modified 15:44–15:46, before the fix was recorded at 15:51. All 4 are green in the gate run.

**2. Cases added.** None. I wrote one probe, a Mermaid concurrency test with 4 cases using the real Mermaid 12.1.0. All 4 were green on the first run that worked, so I moved it out of the worktree into scratch.

**3. Gate.** `cd web && pnpm check` exited 0: `Test Files 17 passed (17)`, `Tests 124 passed (124)`, vite build ✓. The 124 matches the round-1 gate log (`r1-web-gate.log`).

**4. Finding (on the working tree)**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `web/src/api/client.ts:83` | INFEASIBLE | introduced | the spec source names a spec at the repository root `.` (`crates/repoview-sources/src/spec.rs:82,106-109`); story:spec-pages says `{root}` must match a detected root exactly; `apiGet('spec/roots/./ir')` is refused and `client.test.ts` asserts that refusal; the browser's URL parser also turns `%2e` into `.` and removes it, so the repository-root spec can never be fetched | a repository with `system.yaml` or `ess-inputs.yaml` at its top level; none of the repositories under ~/beyond10x has one |

Fix: it belongs in story:spec-pages, which needs a different encoding for the root, not in `apiGet`.

**5. Attacked and could not break.** Path refusal: 23 inputs through a copy of `encodePath`, both modes, none left `/api/spec/roots/` or `/data/spec/roots/` (double encoding, U+2024/U+2025/U+FF0E, mixed case `%2E%2e`, `;` parameters, `?`/`#` in a segment, `\`, tab, space, NUL, zero-width space, malformed `%`, overlong `%C0%AE`, `..%2F..`). Mermaid cleanup with real Mermaid: 3 concurrent failing renders, bad → bad → draw-failing re-render, good → bad, unmount mid-render — no leftovers. `suppressErrorRendering` is in Mermaid's `secure` list (`chunk-VPRB5NB3.mjs:4688`), so `%%{init}%%` cannot switch it back.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/page-frame/scratch/p2-probe.mjs`, `p2-MermaidView.concurrent.probe.test.ts`, `p2-pnpm-check.log`.

```findings
- file: web/src/api/client.ts
  line: 83
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "the spec source names the repository-root spec `.` (spec.rs:82) and story:spec-pages requires `{root}` to match it exactly, but apiGet refuses a `.` segment and no fetch URL can carry one, so that root is unreachable; the fix belongs in the spec-pages wire contract"
```
