---
format: aep.planning-md/3
id: review-result:shell-design-round-2
kind: review-result
status: active
title: plan-critic-design, round 2, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
revision: 1
---
needs-revision

story:server-skeleton — acceptance 10 cannot pass until `story:web-skeleton` has merged, yet no edge records that order and the Design notes say "this story does not depend on" `web/dist`, so the story is both independent and not closable alone; the seam is an acceptance that lives in one body and needs the other, and the fix is to move acceptance 10 to `epic:shell`, which already holds the same "Overview shows Vcs `Present`" criterion, or to add `depends_on story:web-skeleton` and drop the independence claim — .engineering/planning/story/server-skeleton.md:65 and .engineering/planning/story/server-skeleton.md:124

Round-1 findings, one by one:
- **Finding 1 (the `web/dist` order edge): resolved.** Acceptance 7 now serves from an injected asset set behind a trait. Acceptance 8 asserts only that a release build fails when `web/dist` is absent, so neither needs the web story's output.
- **Finding 3 (token transport split across the two bodies): resolved.** "Token transport" now sits in the shared Wire contract section. `story:web-skeleton` acceptance 4 and its Wire contract section cite it.
- **Finding 2 (no acceptance runs the real SPA against `repoview open`): only partly resolved.** Acceptance 10 now covers it, but with the dependency left in prose, which is the finding above.

**What you read:** 4 artifacts (`story:server-skeleton`, `story:web-skeleton`, `epic:shell`, `review-result:shell-design-round-1`) via `aep plan artifact show`, `architecture-design:repoview` by grep only, `relations`, `graph` (all 38 declared edges, including those outside the set) and `validate`. There is no cycle and no chain: the only edges on the two stories are `decomposes epic:shell` and `serves vision:repoview`. `validate` reported "valid".

**What you could not establish:**
- Unverified hypothesis, not a finding: `story:web-skeleton` acceptance 5 proxies `/api` in dev, while the server refuses any `Host` other than `127.0.0.1:&lt;port&gt;` or `localhost:&lt;port&gt;`. Vite's proxy keeps the browser's `Host` header unless `changeOrigin` is set, so `task dev` might get 403s. I did not run it and no acceptance covers it. This is acceptance-lane or implementation territory.
- Out of my lane (scope): `Taskfile.yml` is in neither story's `scope` list. The bodies assign it to the coordinator, so I left it.
- Out of my lane (parallel safety): both stories restate the Wire contract and the dev port 7480. I did not judge whether they can run at the same time.
- I read only the port and token lines of `architecture-design:repoview`.

```findings
- file: .engineering/planning/story/server-skeleton.md
  line: 65
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 10 cannot pass until story:web-skeleton has merged, yet no edge records that order and the Design notes say this story does not depend on web/dist, so the story is both independent and not closable alone; move acceptance 10 to epic:shell, which already holds the same Overview criterion, or add depends_on story:web-skeleton and drop the independence claim"
```
