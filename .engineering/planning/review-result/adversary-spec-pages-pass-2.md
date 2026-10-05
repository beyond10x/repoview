---
format: aep.planning-md/3
id: review-result:adversary-spec-pages-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:spec-pages
relations:
- reviews: story:spec-pages
revision: 1
---
```
unit: story:spec-pages, uncommitted working tree on base 59771f6 (~/.local/state/worktree/trees/b10x/repoview/impl-spec-pages), after correction round 1
verdict: CONFIRMED
cases: executed Rust 95→95, web 339→340, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory of scratch, listed in part 6
needs-coordinator: none
```

The round-1 corrections hold. The new red case is in the web page: every command outcome with a `when` condition shows "when when" and leaves out the condition. All 5 ESS examples produce such outcomes (8 in total).

**1. Diff.** Only the implementor's 5 tracked files (608 insertions, 1 deletion). I added `web/src/components/spec/CommandCard.adversary.test.ts`; no implementation file changed.

**2. Case added.** Mounts `CommandCard` with `billing.invoice.CreateInvoice` from the billing fixture; checks the `accepted` outcome shows `amount.amount > 0` and does not read "when when". Red:
```
AssertionError: expected 'accepted when whencreates Invoice emi…' to contain 'amount.amount > 0'
Received: "accepted when whencreates Invoice emits InvoiceCreatedThe invoice is created in Draft."
 ❯ src/components/spec/CommandCard.adversary.test.ts:20:29
 Test Files  1 failed (1)   Tests  1 failed (1)
```

**3. Suite runs:** `cargo test -p repoview -p repoview-sources --locked` 95 passed, 0 failed, exit 0; `pnpm check` `Tests 1 failed | 339 passed (340)`, exit 1; before count with the case excluded `Tests 339 passed (339)`.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `web/src/api/spec.ts:273` (renders at `web/src/components/spec/CommandCard.vue:28`) | CONFIRMED | introduced | ess 0.52.0 writes a `when` condition as `{kind:"when", predicate:"…"}` with no `cause`; the normaliser keeps only `kind`/`cause`, so the card shows "when when". `normalizeIr` over fresh compile output from all 5 roots in `~/beyond10x/ess/examples`: the only real data dropped that the pages need, 8 outcomes | Any root page with a `when` outcome. Fix: carry `condition.predicate` and show it |
| `web/src/components/spec/IrSections.vue:144` | CONFIRMED | introduced | `views.*.filter` and `order_by` are real in billing, gatepass and oracle-fixture and not shown; acceptance 3 does not name them, so no case | Views section of those roots |
| `crates/repoview-sources/src/lib.rs:21` | CONFIRMED | introduced | `pub use spec::detected_roots;` is outside the story's Scope and the brief lists `lib.rs` as not the unit's | Scope check at merge |

**5. Attacked, could not break:** pass-1 file unchanged and green; `api_spec.rs` stronger (stub logs `--version`, asserts no `ess` call); `detected_roots` and the snapshot both call `roots(env, LIMITS)` with the same root and PATH, so they agree outside Git, git missing, truncated walk, ignored directories; 404 before `find_tool`; `/roots` with none detected answers `[]` without `ess`; normaliser keeps all entity/command/event/view/component/domain keys, field type_refs, relations, identity, lifecycle, emits, errors, subjects, graph nodes/edges/groups/labels across 5 examples; committed billing fixture identical to fresh output after `jq -S`; 41 parallel validates 0.14 s.

**6. Paths written outside the worktree:** `~/.cache/repoview-wave-two/spec-pages/scratch/pass2/` (compile/graph captures, `ess-roots.txt`, a probe test, logs). Lease `adversary-spec-pages-2` taken and released.

```findings
- file: web/src/api/spec.ts
  line: 273
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "normalizeIr drops condition.predicate, so every ess `when` outcome renders as \"when when\" with no condition (8 outcomes across all 5 ess examples)"
- file: web/src/components/spec/IrSections.vue
  line: 144
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "view filter and order_by from real ess output are dropped and not shown in the Views section"
- file: crates/repoview-sources/src/lib.rs
  line: 21
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the correction re-exports detected_roots from lib.rs, a file outside the story's Scope and named not-yours in the brief"
```
