---
format: aep.planning-md/3
id: review-result:shell-design-round-1
kind: review-result
status: active
title: plan-critic-design, round 1, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
revision: 1
---
needs-revision
story:server-skeleton — acceptance 4 (`GET /` serves "the embedded `index.html`") and 7 (release build needs `web/dist/index.html` and names `task build`) depend on the `web/dist` output that story:web-skeleton acceptance 2 produces, and no edge records that order; add `depends_on story:web-skeleton`, or restate acceptance 4 against the debug built-in page — .engineering/planning/story/server-skeleton.md:47
story:server-skeleton — the set is cut Rust against Vue, and no acceptance runs the built SPA against a real `repoview open`; the server story stops at serving `index.html` bytes and the web story renders hand-written fixtures, so epic:shell's "Overview shows Vcs `Present`" is checked in neither and the integration error would sit in the seam; add an acceptance here (with the edge above) that serves the built `web/dist` and renders the real snapshot, or add a third integration story — .engineering/planning/story/server-skeleton.md:43-48 and .engineering/planning/story/web-skeleton.md:26-34
story:web-skeleton — acceptance 4 names the `token` query parameter and the `X-Repoview-Token` header, which are defined only in the server story (acceptance 4 and the Design notes token line), while the section both stories call the shared Wire contract covers the JSON shape only, so the auth protocol is half in each body; move the token transport into the Wire contract section and have both stories cite it — .engineering/planning/story/web-skeleton.md:34 and .engineering/planning/story/server-skeleton.md:100

**What you read:** 4 artifacts (story:server-skeleton, story:web-skeleton, epic:shell, architecture-design:repoview) via `aep plan artifact show`, plus `relations`, `graph` and `validate`. I walked all 28 declared edges across the 12 artifacts, outside the set as well as inside it. There is no cycle. The only edges on the two stories are `decomposes epic:shell` and `serves vision:repoview`, so there is no chain either. `validate` reported "valid".

**What you could not establish:**
- Out of my lane (scope): no story owns `Taskfile.yml` (`task build`, `task dev`, `task check`), the wire-types generation, or its drift check. All three are in epic:shell's Scope, but neither story's `scope` lists `Taskfile.yml` or `ess/`. Server acceptance 7 names `task build`, which nothing creates. Grep of `.engineering/planning` found no other owner.
- Out of my lane (parallel safety): both stories describe the Wire contract and the dev port 7480. I did not judge whether they can run concurrently.
- Out of my lane (acceptance): I did not check whether the `aep` `tool_version` and similar acceptance items are testable.
- The truncated middle of architecture-design:repoview (the API table) was not read. It does not affect the findings above.

```findings
- file: .engineering/planning/story/server-skeleton.md
  line: 47
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: acceptance 4 (`GET /` serves "the embedded `index.html`") and 7 (release build needs `web/dist/index.html` and names `task build`) depend on the `web/dist` output that story:web-skeleton acceptance 2 produces, and no edge records that order; add `depends_on story:web-skeleton`, or restate acceptance 4 against the debug built-in page
- file: .engineering/planning/story/server-skeleton.md
  line: 43
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the set is cut Rust against Vue, and no acceptance runs the built SPA against a real `repoview open`; the server story stops at serving `index.html` bytes and the web story renders hand-written fixtures, so epic:shell's "Overview shows Vcs `Present`" is checked in neither and the integration error would sit in the seam; add an acceptance here (with the edge above) that serves the built `web/dist` and renders the real snapshot, or add a third integration story
- file: .engineering/planning/story/web-skeleton.md
  line: 34
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: acceptance 4 names the `token` query parameter and the `X-Repoview-Token` header, which are defined only in the server story, while the section both stories call the shared Wire contract covers the JSON shape only, so the auth protocol is half in each body; move the token transport into the Wire contract section and have both stories cite it
```
