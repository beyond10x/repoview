---
format: aep.planning-md/3
id: review-result:shell-scope-round-2
kind: review-result
status: active
title: plan-critic-scope, round 2, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
- reviews: epic:shell
revision: 1
---
needs-revision

epic:shell — "`repoview` run in a project directory opens a browser tab on an Overview page" is promised, and the design maps bare `repoview` to `repoview open`. Neither story's body or acceptance claims a default subcommand, since all of them use `repoview open`. Add one acceptance, or a design note on the clap default, to story:server-skeleton, which is the natural owner. — .engineering/planning/epic/shell.md:16 (design line 85 of `aep plan artifact show architecture-design:repoview`; story at .engineering/planning/story/server-skeleton.md:11-12 and 38)

Round-1 findings, all checked against the revised bodies:

| Round-1 finding | Status | Evidence |
|---|---|---|
| PWD discovery unclaimed | fixed | `story:server-skeleton` acceptance 2 |
| `Taskfile.yml`, `task build`, `task dev` unclaimed | fixed | epic scope last bullet names the coordinator as author. The story's design notes record that it exists on the integration branch before dispatch. |
| Two UNMAPPED markers | fixed | epic Nouns settles the payload marker and defers the views marker to the wire-types story. `ess/domains/project.yaml:67` still holds the views marker, which matches that deferral. |
| Token and static-asset narrowing | fixed | the epic acceptance now says static assets need no token. The story's Token transport section and design lines 108-109 agree. |
| Static mode is reach beyond the parent | fixed | the epic scope now lists the static mode of the data client |

What I read: 5 artifacts (`epic:shell`, both stories, `architecture-design:repoview`, `review-result:shell-scope-round-1`). Commands: `aep plan artifact show` on each, `aep plan artifact graph`, and `grep` on the design and `ess/domains/project.yaml`. I extracted 13 promises from the epic and traced 12 to an item. The untraced one is the bare-`repoview` finding above. The wire types generated from `ess/` and the `task check` drift check are not counted as a gap, because the epic and the story's Out of scope both defer them to a later story.

Could not establish: none for scope.

Out of my lane:
- `story:server-skeleton` acceptance 10 depends on a browser check, which is for the acceptance critic.
- `story:web-skeleton` acceptance 5 hard-codes port 7480, which is for the design critic.

```findings
- file: .engineering/planning/epic/shell.md
  line: 16
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: '"`repoview` run in a project directory opens a browser tab" is promised, and no story claims bare `repoview` defaulting to `open` (design line 85); every acceptance uses `repoview open`, so story:server-skeleton should take it'
```
