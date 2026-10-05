---
format: aep.planning-md/3
id: review-result:adversary-web-skeleton-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:web-skeleton
relations:
- reviews: story:web-skeleton
revision: 1
---
unit: story:web-skeleton, uncommitted working tree at ~/.local/state/worktree/trees/b10x/repoview/impl-web-skeleton (base d411e1c)
verdict: NEEDS-CHANGE
cases: executed 16→17, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: none

**1. What I touched.** `git --no-pager diff --stat` prints nothing, because everything under `web/` is untracked. `git status --short` shows `?? web/`, which now holds 22 untracked files: the implementor's 21 plus mine. My only file is a test file:
`web/src/components/SourceCard.adversary.test.ts`. I changed no implementation file. I ran `prettier --write` on that file only.

**2. The case I added.** On a detached HEAD the vcs card shows no empty summary value, and it still shows the 7-character head.

- **Red now.** I ran it on its own, before the full suite: `pnpm exec vitest run src/components/SourceCard.adversary.test.ts`.
  ```
  × a detached HEAD (summary.branch "") renders no empty summary value
  AssertionError: vcs card "branch" value: expected '' not to be '' // Object.is equality
   Test Files  1 failed (1)
        Tests  1 failed (1)
  ```
- **What was measured:** `SourceCard.vue:38` adds `['branch', '']`, so the card shows a "branch" label with an empty value.
- **What reaches it:** the server's `vcs.rs:48/60` sends `"branch": branch.trim()` from `git branch --show-current`. That command prints nothing on a detached HEAD; I checked this in a scratch repository and deleted it afterwards. CI checkouts, `git bisect` and a rebase in progress all leave HEAD detached.
- **Suggested fix (not applied):** show `detached HEAD` when `summary.branch === ''`, or leave the row out. The case passes with either.

**3. The suite.** `cd web && pnpm check` exits 1. Prettier, ESLint and vue-tsc pass. Vitest fails, so `vite build` never ran:
```
 Test Files  1 failed | 3 passed (4)
      Tests  1 failed | 16 passed (17)
[ELIFECYCLE] Command failed with exit code 1.
EXIT=1
```

**4. Findings** (all against the uncommitted tree; `web/` does not exist at d411e1c, so every origin is introduced)

| file:line | verdict | origin | finding | what reaches it |
|---|---|---|---|---|
| web/src/components/SourceCard.vue:38 | NEEDS-CHANGE | introduced | A detached HEAD shows an empty branch value | Any detached checkout (item 2) |
| web/src/api/snapshot.ts:63 | CONFIRMED | introduced | No test covers a 403 in static mode. If `&& !isStatic` is deleted, the suite stays green, and a static page would then wrongly tell the user to reopen the URL repoview printed | A static host that answers 403 for `data/snapshot.json` |
| web/src/components/SourceCard.vue:72 | CONFIRMED | introduced | A Failed card never names its tool or version, although the story says each card shows "the tool and version that answered" | The server sets `tool` and `tool_version` before marking vcs Failed (`lib.rs` `with_tool_version`). Acceptance item 3 only requires "failed" plus the diagnostic, so this is a note |

**5. What I attacked and could not break**
- **XSS:** there is no `v-html` or `innerHTML` anywhere. Every snapshot string is shown through `{{ }}` or an attribute binding.
- **Token:** it never reaches a log or an error message. It is removed from the URL before vue-router reads the location, and static mode sends no token header.
- **An unknown availability value** from a newer server shows `unknown availability: <value>`, never an empty card.
- **Empty diagnostic:** an empty `diagnostic` cannot happen, because the server's `process.rs:78` substitutes an "exited with" message.
- **ToolMissing with a null tool** shows `tool missing: unknown tool`.
- **Unborn branch:** `head` null leaves out the head row without crashing.
- **Wire contract:** the client's types match `story:server-skeleton` field for field.
- **Dev proxy and build:** the proxy, `base: "./"` and the build-output assertions all hold.

**6. Paths written outside the worktree**
- `~/.cache/repoview-wave-one/web-skeleton/scratch/adversary-check.log`, the log of the full `pnpm check` run.
- I also created a temporary git repository at `.../scratch/detached` and deleted it.
- My worktree lease `adversary-web-skeleton` is acquired and released.
- `df -h /` showed 9.4G free, under the 10G rule. I only ran vitest and the web gate, no cargo build.

```findings
- file: web/src/components/SourceCard.vue
  line: 38
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "summary.branch \"\" from a detached HEAD (vcs.rs branch.trim()) renders a branch row with an empty value"
- file: web/src/api/snapshot.ts
  line: 63
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: deleting the static-mode exclusion from the 403 token-rejected branch leaves the suite green, because no case covers a 403 in static mode
- file: web/src/components/SourceCard.vue
  line: 72
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a Failed card shows neither its tool nor its tool_version, although the story says every card shows the tool and version that answered
```
