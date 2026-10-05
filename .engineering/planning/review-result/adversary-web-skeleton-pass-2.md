---
format: aep.planning-md/3
id: review-result:adversary-web-skeleton-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:web-skeleton
relations:
- reviews: story:web-skeleton
revision: 1
---
unit: story:web-skeleton, uncommitted working tree at ~/.local/state/worktree/trees/b10x/repoview/impl-web-skeleton (base d411e1c)
verdict: CONFIRMED (3 red cases, all `note` severity; no blocker)
cases: executed 49→52, red 3
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (scratch logs; listed in part 6)
needs-coordinator: whether the two `note` findings should hold the unit or go to a follow-up story

The three pass-1 fixes still work, and I found two new defects, both small. My three new cases fail on purpose. Before is 49 because that is the implementor's round-1 gate count (`scratch/gate-r1.log`). I did not run the suite first.

**1. What I touched.** `git --no-pager diff --stat` prints nothing, because all of `web/` is untracked. I added two test files and changed no implementation file:
- `web/src/components/SourceCard.adversary-2.test.ts` (new)
- `web/src/api/snapshot.adversary.test.ts` (new)

The pass-1 file `web/src/components/SourceCard.adversary.test.ts` is unchanged. Its mtime is 14:50:57, before the pass-1 check log at 14:51. Run alone it passes: `Tests 1 passed (1)`.

**2. Cases added.** I ran them alone before the suite. All three fail.

| Case | Asserts | Red output (verbatim) |
|---|---|---|
| SourceCard.adversary-2 "summary.head null (unborn branch)…" | A vcs card with `head: null` shows the head row as "no commit" | `AssertionError: vcs summary labels: expected [ 'branch', 'worktree' ] to include 'head'` |
| snapshot.adversary "captureToken() does not throw…" | With `sessionStorage` throwing SecurityError, captureToken does not throw, strips the token from the URL, and the token still reaches `/api/snapshot` | `expected [Function] to not throw an error but DOMException{ stack: 'SecurityError:…' } was thrown` |
| snapshot.adversary "loadSnapshot() settles…" | With storage blocked, loadSnapshot returns a state instead of rejecting | `promise rejected "DOMException{ stack: 'SecurityError:…' }" instead of resolving`, at `loadSnapshot src/api/snapshot.ts:59:42` |

**3. Suite.** I ran `cd web && pnpm check` after writing the cases. Prettier, ESLint and vue-tsc pass; vitest fails, so `vite build` did not run.
```
 Test Files  2 failed | 5 passed (7)
      Tests  3 failed | 49 passed (52)
[ELIFECYCLE] Command failed with exit code 1.
EXIT=1
```

**4. Findings.** All are against the uncommitted tree. `web/` does not exist at d411e1c, so both are `introduced`.

| file:line | verdict | origin | finding | what reaches it |
|---|---|---|---|---|
| web/src/components/SourceCard.vue:63 | CONFIRMED | introduced | The "no commit" fallback only fires when `head === ""`, which the server never sends. On a repository with no commits the server sends `head: null`, and the card drops the head row instead. That breaks the promise in `display.ts` that a missing value shows its fallback. Fix: show `no commit` when `head` is null. | Any repository between `git init` and its first commit. `vcs.rs` sends `head: None` when `rev-parse --verify --quiet HEAD` fails; in scratch it exited 1 while the branch printed `main`. |
| web/src/api/snapshot.ts:46 and :59 | CONFIRMED | introduced | Neither function catches a throw from `sessionStorage`. `captureToken()` runs before `createApp` in `main.ts`, so the page stays blank with no message. `loadSnapshot` reads storage outside its `try`, so it rejects, and `useSnapshot` has no catch. Fix: wrap storage access and keep the token in memory as a fallback. | A browser with all site data blocked (Chrome "Block all cookies"), where reading `window.sessionStorage` throws a SecurityError. Uncommon. |

Judgement notes (no case written):
- **Mutant not caught (`useSnapshot.ts:18`):** delete `provide(SNAPSHOT_KEY, state)` and the suite stays green while the page fetches the snapshot twice. No test counts fetches at App level.
- **Mutant not caught (`SourceCard.vue:29`):** remove the `ToolMissing` exclusion and the suite stays green, while the card shows "aep · version unknown" under "tool missing: aep".
- **Fixtures do not match the server:** for Absent sources the server sends `tool` set to the tool's name, but the fixtures send `null`. So with real data an Absent quality card shows "codegate · version unknown", which reads as if the tool answered.

**5. Attacked, could not break:**
- Pass-1 corrections: the 15 original test names from `red-run.log` all still exist, and the assertions that appear there are still present or stronger.
- The 403 handling in static mode, at both unit and App level.
- Failed cards: they show tool, version and diagnostic for each way the server builds a Failed source.
- The `orElse`/`hasText` changes: the server trims or replaces whitespace, so a whitespace-only string cannot arrive.
- XSS: there is still no `v-html` or `innerHTML`.
- Token edge cases: an empty `?token=`, a repeated token parameter, and a server restart all end in the token-rejected message.
- Dev proxy test: it still catches `changeOrigin` being removed.

**6. Paths written outside the worktree:**
- `~/.cache/repoview-wave-one/web-skeleton/scratch/adversary-p2-red.log`
- `~/.cache/repoview-wave-one/web-skeleton/scratch/adversary-p2-check.log`
- `.../scratch/unborn-p2`: a temporary git repository, already deleted.

My lease `adversary-web-skeleton-pass-2` was acquired and has been released. `df -h /` showed 12G free.

```findings
- file: web/src/components/SourceCard.vue
  line: 63
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "summary.head null from an unborn branch (vcs.rs head None) drops the head row; the 'no commit' fallback only fires for \"\", which the server never sends"
- file: web/src/api/snapshot.ts
  line: 46
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a throwing sessionStorage (site data blocked) makes captureToken throw before createApp, leaving a blank page, and makes loadSnapshot reject, leaving useSnapshot stuck at loading
```
