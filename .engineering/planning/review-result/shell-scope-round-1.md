---
format: aep.planning-md/3
id: review-result:shell-scope-round-1
kind: review-result
status: active
title: plan-critic-scope, round 1, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
- reviews: epic:shell
revision: 1
---
needs-revision

epic:shell — "Project discovery: Git top level from `PWD`, else `PWD`; `--root` overrides." is promised and no item claims it. Every acceptance runs with `--root`, and neither story mentions PWD discovery, so `story:server-skeleton` should take it. — .engineering/planning/epic/shell.md:24
epic:shell — "`task build`, `task dev`, `task check`" are promised. Only `task check` wiring is named as deferred, and no item creates `Taskfile.yml` or claims `task build` or `task dev`. `story:server-skeleton` acceptance 7 and its debug fallback page both depend on a `task build` that nothing builds. — .engineering/planning/epic/shell.md:26-27
epic:shell — "Its two `UNMAPPED:` markers are settled here before the first story." No item claims settling them. `story:server-skeleton` adds `tool`, `tool_path` and `summary` to the wire shape without touching `ess/domains/project.yaml`, which is outside its scope, and the markers say the first story settles the wire model. — .engineering/planning/epic/shell.md:41 (also ess/domains/project.yaml:61-65)
story:server-skeleton — the epic promises "A request without the run token, or with `Host: evil.example`, is refused with 403". The story exempts static assets from the token and tests the token only on `/api/*`, a narrowing nothing records against the epic. The design says "Every request must carry the run token". — .engineering/planning/story/server-skeleton.md:101
story:web-skeleton — static mode (the `repoview-mode` meta, `./data/snapshot.json`, hash history, `base: "./"`) traces to no sentence in `epic:shell`. It is the outcome of `epic:static-export`, which says it extends this acceptance, so it is reach beyond the parent. Either move it to that epic's story or add it to `epic:shell`'s scope. — .engineering/planning/story/web-skeleton.md:37

Read: 4 artifacts (the two stories, `epic:shell`, `architecture-design:repoview`), plus the `epic:static-export` body, the `ess/domains/project.yaml` markers and the graph. I ran `aep plan artifact show` for each of the four, `aep plan artifact graph`, and `grep`/`cat -n` on the files above. I extracted 12 promises from `epic:shell`: 3 outcome, 5 scope with the wire-types bullet, 3 acceptance, 1 nouns. 9 are traced to an item. 1 is deliberately deferred and not counted as a gap: the wire types generated from `ess/` and the drift check in `task check`. The 3 findings above are the gaps. I did not count the "listing every source with producer" promise separately, because the web story covers it.

Could not establish: none for scope. Out of my lane:
- `story:server-skeleton` acceptance 4 has no check that `repoview open` launches the browser. That is the acceptance critic's.
- `story:web-skeleton` acceptance 5 hard-codes port 7480 against the design's `task dev`. That is the design critic's.

```findings
- file: .engineering/planning/epic/shell.md
  line: 24
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: '"Project discovery: Git top level from PWD, else PWD; --root overrides" is promised and no item claims it; every acceptance uses --root (story:server-skeleton would take it)'
- file: .engineering/planning/epic/shell.md
  line: 26
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: '"task build, task dev, task check" are promised; only task check wiring is named as deferred, and no item creates Taskfile.yml or claims task build/task dev, which story:server-skeleton acceptance 7 depends on'
- file: .engineering/planning/epic/shell.md
  line: 41
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the epic's two UNMAPPED markers in ess/domains/project.yaml are to be settled by the first story, but no item claims it and story:server-skeleton scope excludes ess/ while adding tool, tool_path and summary to the wire shape
- file: .engineering/planning/story/server-skeleton.md
  line: 101
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the epic promises a request without the run token is refused with 403, but the story exempts static assets from the token and tests it only on /api/*, a narrowing recorded nowhere against the epic
- file: .engineering/planning/story/web-skeleton.md
  line: 37
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: static mode (repoview-mode meta, ./data/snapshot.json, hash history, base "./") traces to no sentence in epic:shell and belongs to epic:static-export; move it or add it to the epic's scope
```
