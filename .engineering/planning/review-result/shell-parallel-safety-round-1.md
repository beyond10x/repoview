---
format: aep.planning-md/3
id: review-result:shell-parallel-safety-round-1
kind: review-result
status: active
title: plan-critic-parallel-safety, round 1, epic:shell stories
relations:
- reviews: story:server-skeleton
- reviews: story:web-skeleton
revision: 1
---
needs-revision
story:server-skeleton — it reads `web/dist`, which only `story:web-skeleton` produces (its scope is `web/`) and `.gitignore` excludes, and it neither says how it builds without that directory nor records the dependency; the body must either add `allow_missing` to the `RustEmbed` derive (or a placeholder `web/dist`) or get an ordering edge from `story:web-skeleton` naming `web/dist` as the reason (cited for the server side, inferred that the dist is absent in a fresh worktree) — .engineering/planning/story/server-skeleton.md:96

The two declared scopes are disjoint, and the CLI's wave 1 reports `collisions: []`. The finding rests on the `web/dist` read, which the scope list does not show.

Parts 3 and 4:

- **Read:** 2 artifacts, all by the commands below.
  - `aep plan artifact show story:server-skeleton` and `aep plan artifact show story:web-skeleton`
  - `aep plan artifact list --format json`
  - `aep plan artifact waves --kind story --status draft --format json`
  - `.gitignore`, and `rust-embed-impl-8.12.0/src/lib.rs:444-489` in the local cargo registry
- **Surfaces:**
  - Cited: 2 items, `Cargo.lock`, `Cargo.toml`, `crates/repoview-sources/`, `crates/repoview/`, `rust-toolchain.toml` for the server and `web/` for the web.
  - Inferred: 0.
  - Not placed: 0.
  - The `web/dist` surface in the server body is cited, but it is not in the server's declared scope.
- **Not established:**
  - Whether the pinned rust-embed version is the one 8.12.0 I checked. The body names no version.
  - `web/` does not exist yet, so I could not check the build output.
  - Out of my lane: whether acceptance 4 (`GET /` returns 200) can be checked on a debug build with no `web/dist`. The body's "built-in one-line HTML page" implies it can, but that depends on the compile question above. This belongs to the acceptance critic.
  - Coordinator-owned `Taskfile.yml`, `.gitignore` and `AGENTS.md` are outside the two scopes, so neither story claims them.

```findings
- file: .engineering/planning/story/server-skeleton.md
  line: 96
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: it reads web/dist, which only story:web-skeleton produces (its scope is web/) and which .gitignore excludes, and it neither says how it builds without that directory nor records the dependency; the body must either add allow_missing to the RustEmbed derive (or a placeholder web/dist) or get an ordering edge from story:web-skeleton naming web/dist as the reason (cited for the server side; the absence of web/dist in a fresh worktree is inferred)
```
