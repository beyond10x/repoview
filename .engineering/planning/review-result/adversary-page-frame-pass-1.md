---
format: aep.planning-md/3
id: review-result:adversary-page-frame-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:page-frame
relations:
- reviews: story:page-frame
revision: 1
---
```
unit: story:page-frame at 59771f6 (impl/page-frame, base d8ce444), worktree ~/.local/state/worktree/trees/b10x/repoview/impl-page-frame
verdict: CONFIRMED (2 failing cases, both introduced)
cases: executed 111→120 web (Rust 49→49), red 3
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 (scratch logs; listed in part 6)
needs-coordinator: none
```

**1. What I touched.** `git diff --stat` is empty: I changed no tracked file. `git status --short` shows four new files, all of them tests:
```
?? web/src/api/client.adversary.test.ts
?? web/src/components/MarkdownView.adversary.test.ts
?? web/src/components/MermaidView.adversary.test.ts
?? web/src/components/MermaidView.sanitise.adversary.test.ts
```

**2. Cases added.** I ran each file on its own before running the suite.

| File | What it asserts | Now |
|---|---|---|
| `client.adversary.test.ts` | the router turns `/specs/..%2F..%2Fsnapshot` into the param `../../snapshot` (shows what can reach `apiGet`) | green |
| same, server mode | `apiGet('spec/roots/<id>/ir')` with id `..`, `.` or `../..` either requests a path under `/api/spec/roots/` or makes no request | **red** |
| same, static mode | the same check, under `/data/spec/roots/` | **red** |
| `MermaidView.adversary.test.ts` | with the real Mermaid 12.1.0, a parse error shows the `<pre>` and leaves nothing outside the component | **red** |
| `MarkdownView.adversary.test.ts` (3 cases) | none of these survive: `vbscript:`, `data:text/html`, `data:image/svg`, reference links, entity or tab obfuscation, linkify attribute breakout, raw svg/math/form/style/iframe, a fence info string injecting attributes | green |
| `MermaidView.sanitise.adversary.test.ts` (2 cases) | the second DOMPurify pass removes `javascript:` in `href`/`xlink:href`, `<set>`/`<animate>` hrefs, handlers in foreignObject, nested math/style mXSS; it keeps Mermaid's `<style>` and HTML labels | green |

Red output, verbatim (abridged):
```
AssertionError: id ".." requested /api/spec/ir: expected '/api/spec/ir' to match /^\/api\/spec\/roots\//
AssertionError: id ".." requested /data/spec/ir.json: expected '/data/spec/ir.json' to match /^\/data\/spec\/roots\//
AssertionError: expected [ 'div#drepoview-mermaid-1' ] to deeply equal []
 Test Files  2 failed (2)   Tests  3 failed | 1 passed (4)
```

**3. Suite runs, after the cases existed.**
- `cargo test -p repoview --locked`: exit 0, 49 passed across 9 binaries. `--list` confirms the three `api::tests::*` tests come from this tree.
- `pnpm exec vitest run` with my four files excluded: `Tests 111 passed (111)`, exit 0. This is the "before" count.
- `pnpm check`: prettier, eslint and vue-tsc pass. Vitest reports `Test Files 2 failed | 15 passed (17)` and `Tests 3 failed | 117 passed (120)`, exit 1. `vite build` did not run because the chain stopped at vitest.

**4. Findings** (both cover 59771f6)

**F1. `apiGet` lets a `.` or `..` id segment escape its prefix.** `web/src/api/client.ts:72`, NEEDS-CHANGE, introduced, warning. `encodeURIComponent` leaves `.` and `..` unchanged and `fetch` treats them as dot-segments, so `spec/roots/../ir` requests `/api/spec/ir`; in static mode it can climb out of `./data/`. Reached through the `/specs/:root(.*)` and `/plan/artifact/:id` routes; vue-router decodes `%2F` and `%2E`, so a link in repository markdown supplies the id. Fix: refuse any empty, `.` or `..` segment with `error`.

**F2. A failed Mermaid render leaves a stray error diagram on `document.body`.** `web/src/components/MermaidView.vue:36`, NEEDS-CHANGE, introduced, warning. `initialize` does not set `suppressErrorRendering: true`; Mermaid 12.1.0 (`mermaid.core.mjs:1343-1404`) draws its "Syntax error" diagram into `div#d<id>` on `document.body` and throws before `removeTempElements()`. Reached by any invalid diagram source a future page renders. Fix: `suppressErrorRendering: true`. The shipped tests mock Mermaid, so they cannot see this.

**5. Attacked, could not break.** MarkdownView: all schemes and markup in part 2. Mermaid second pass matches strict-mode config; error `<pre>` uses text interpolation. Rust registry: guard wraps merged module routes; a duplicate `/api/snapshot` panics at build time; nothing enforces that module routes start with `/api/` (judgement only, not raised). Nav dimming source ids match the Rust `id()` values.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/page-frame/scratch/vitest-deselected.log`, `~/.cache/repoview-wave-two/page-frame/scratch/pnpm-check.log`. Lease `adversary-page-frame` taken and released.

```findings
- file: web/src/api/client.ts
  line: 72
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "encodePath leaves `.` and `..` segments unencoded, so an id of `..` from /specs/:root(.*) or /plan/artifact/:id makes fetch request a document outside /api/<prefix> or ./data/<prefix>"
- file: web/src/components/MermaidView.vue
  line: 36
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "mermaid.initialize omits suppressErrorRendering, so Mermaid 12.1.0 leaves its syntax-error diagram (div#d<id>) on document.body after every failed render"
```
