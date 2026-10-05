---
format: aep.planning-md/3
id: review-result:shell-acceptance-round-1
kind: review-result
status: active
title: plan-critic-acceptance, round 1, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
revision: 1
---
needs-revision

story:server-skeleton — acceptance 5, "no flag or environment variable changes the address", is a universal negative that no run can show, so the check is a vote. It should name a test that passes a conflicting flag or env value, or the `bind` argument, and shows the listener still on `127.0.0.1` — .engineering/planning/story/server-skeleton.md:49
story:server-skeleton — acceptance 4 sends `GET /api/snapshot` without a token to 403, but also says `GET /api/nope` is 404 without saying whether a token is sent. An unauthenticated `/api/nope` gets 403 or 404 depending on guard order, so two testers get different answers; it should say "with a valid token" — .engineering/planning/story/server-skeleton.md:48
story:web-skeleton — acceptance 6 ends "`vite build` uses `base: "./"` so `web/dist` works from any directory". It names no observation. It should say that `web/dist/index.html` references its assets as `./assets/…` (or that `vite.config.ts` sets `base: "./"`), and that is a second outcome beside static mode in the same item — .engineering/planning/story/web-skeleton.md:39

**What I read:** 2 stories, with the parent epic:shell and `architecture-design:repoview` read for context only. I ran `aep plan artifact show` on each, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, and `cat -n` on both story bodies.

**What I could not establish:**
- Acceptance 3 of server-skeleton (`aep` on `PATH`) can only be checked where `aep` is installed. That is an environment condition and I did not count it as a defect.
- I read both numbered acceptance lists as one statement each, not as multiple independent ones. Every item is a separately checkable pass/fail, and the story is done only when all items pass. I did not flag the lists as "more than one statement", and the caller may read the rubric more strictly.
- Out of my lane, so it did not set my verdict:
  - `epic:shell` scope lists "Wire types generated from `ess/`; drift check", but server-skeleton § Out of scope defers it (scope critic).
  - The wire contract is duplicated by reference between the two stories (design and parallel-safety critics).

```findings
- file: .engineering/planning/story/server-skeleton.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: acceptance 5, "no flag or environment variable changes the address", is a universal negative that no run can show, so the check is a vote; it should name a test that passes a conflicting flag or env value and shows the listener still on 127.0.0.1
- file: .engineering/planning/story/server-skeleton.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: acceptance 4 says GET /api/nope is 404 without saying whether a token is sent, while a tokenless /api request is 403 on line 45, so an unauthenticated /api/nope has two possible answers; it should say "with a valid token"
- file: .engineering/planning/story/web-skeleton.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: acceptance 6 ends with "vite build uses base ./ so web/dist works from any directory" and names no observation, and it is a second outcome beside static mode; it should state what the built index.html shows (assets referenced as ./assets/) or that vite.config.ts sets base "./"
```
