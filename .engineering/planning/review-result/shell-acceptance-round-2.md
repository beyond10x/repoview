---
format: aep.planning-md/3
id: review-result:shell-acceptance-round-2
kind: review-result
status: active
title: plan-critic-acceptance, round 2, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
revision: 1
---
approve

**What I read:** 2 stories (`story:server-skeleton` revision 7 and `story:web-skeleton` revision 5), plus `epic:shell`, `architecture-design:repoview` and `review-result:shell-acceptance-round-1` for context. Commands: `aep plan artifact show` on each of the four, and `aep plan artifact kinds`. I read `architecture-design:repoview` only as context and did not run `show` on it, so I read the 2 stories and 2 context artifacts in full, not 3.

**Round-1 findings, all three fixed:**
- **Bind address:** server acceptance 6 now sets `HOST`, `BIND`, `REPOVIEW_HOST`, `REPOVIEW_BIND` and `REPOVIEW_ADDR` to `0.0.0.0`. It asserts `local_addr().ip()` is `127.0.0.1`, and requires `--host` and `--bind` to fail with clap's unknown-argument error.
- **`/api/nope`:** server acceptance 5 now reads "`GET /api/nope` with the valid token → 404", so the guard order no longer changes the answer.
- **Relative asset base:** `base: "./"` is now its own item, web acceptance 7, and names an observation. The built `index.html` references `./assets/…`, checked by a vitest case or a build assertion. Static mode is now the only outcome in web acceptance 6.

**What I could not establish:**
- Server acceptance 10 is a browser check by the coordinator. A person can repeat it (the Overview page shows `vcs` `Present` and its branch), but no program does. I did not count that as a defect.
- Server acceptance 9 gives the `doctor` line contents as "tool path and version or the reason" with no exact format. A test can check it by substring, so I left it.
- Both acceptance lists are numbered, with several checks per item, as in round 1. Each check passes or fails separately, so I did not flag them as more than one statement.
- Whether `web/package.json`, the `pnpm check` script and `Taskfile.yml` exist is not something I checked. The acceptance criteria name them as things the work creates.
- Out of my lane, and it did not set my verdict:
  - `epic:shell` scope still lists the wire-types generation and drift check that `story:server-skeleton` § Out of scope defers (scope critic).
  - The wire contract is shared between the two stories by reference (design and parallel-safety critics).
  - The dev proxy port `7480` in web acceptance 5 is not tied to any server default (design critic).

```findings
[]
```
