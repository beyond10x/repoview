---
format: aep.planning-md/3
id: story:page-frame
kind: story
status: implemented
title: 'Shared page frame: API module registry, client, routes, nav, markdown and Mermaid components'
summary: Routing, nav, apiGet with static mode, MarkdownView (markdown-it + DOMPurify), MermaidView, so page stories add only their own files.
relations:
- decomposes: epic:shell
- serves: vision:repoview
- depends_on: story:server-skeleton
- depends_on: story:web-skeleton
scope:
- confidence: cited
  path: crates/repoview/src/api/
- confidence: cited
  path: crates/repoview/src/lib.rs
- confidence: cited
  path: crates/repoview/src/server.rs
- confidence: cited
  path: web/package.json
- confidence: cited
  path: web/pnpm-lock.yaml
- confidence: cited
  path: web/src/App.vue
- confidence: cited
  path: web/src/api/client.ts
- confidence: cited
  path: web/src/api/snapshot.ts
- confidence: cited
  path: web/src/components/MarkdownView.vue
- confidence: cited
  path: web/src/components/MermaidView.vue
- confidence: cited
  path: web/src/components/SourceCard.vue
- confidence: cited
  path: web/src/pages/
- confidence: cited
  path: web/src/router.ts
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:28:40Z", actor: "human:timo", revision: 15}
- {from: "proposed", to: "active", at: "2026-10-05T13:28:40Z", actor: "human:timo", revision: 16}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:15Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Story

As the implementor of a page, I add my page by writing my own files only: an API module, a source
reader, a page component and their tests. The routing, navigation, API client, markdown renderer and
diagram renderer already exist and are shared.

## Acceptance

1. `crates/repoview/src/api/mod.rs` exposes `pub fn routes() -> Router<AppState>` that merges
   `plan::routes()`, `spec::routes()`, `quality::routes()` and `repository::routes()`, each defined in
   its own file under `crates/repoview/src/api/` and returning an empty router. `server::router`
   merges `api::routes()` so every route under it gets the token and Host guard and the `/api` 404
   fallback. A test adds a throwaway route through a test-only module and shows it 403 without the
   token and 200 with it.
2. `web/src/api/client.ts` exports `apiGet<T>(path: string): Promise<ApiResult<T>>`. In server mode it
   fetches `/api/<path>` with `X-Repoview-Token`; in static mode it fetches `./data/<path>.json`
   (path segments URL-encoded, `/` kept) with no token. `snapshot.ts` uses it. A 403 in server mode is
   `token-rejected`; any other non-2xx is `error` with the status; a network failure is `error`.
   Vitest covers each case in both modes.
3. `web/src/router.ts` has the routes `/` (Overview), `/plan` (board), `/plan/tree`,
   `/plan/artifact/:id`, `/specs`, `/specs/:root(.*)`, `/quality`, `/repository`, each pointing at its
   own page file under `web/src/pages/` (`PlanBoardPage.vue`, `PlanTreePage.vue`,
   `PlanArtifactPage.vue`, `SpecsPage.vue`, `SpecRootPage.vue`, `QualityPage.vue`,
   `RepositoryPage.vue`), each a placeholder that renders its title. The left nav lists Overview,
   Plan, Specs, Quality, Repository; a nav entry whose source is `Absent` in the snapshot is shown
   dimmed with "absent", never hidden.
4. `web/src/components/MarkdownView.vue` renders a markdown string with markdown-it (`html: false`,
   `linkify: true`) and sanitises the output with DOMPurify before it reaches the DOM. Vitest: a
   `<script>` tag, an `onerror=` attribute and a `javascript:` link in the input do not appear in the
   rendered DOM; a table and a fenced code block do.
5. `web/src/components/MermaidView.vue` renders a Mermaid source string with
   `securityLevel: "strict"` into an SVG; a render error shows the error text and the source in a
   `<pre>`, never a blank box. Vitest covers the error path (Mermaid mocked).
6. The Overview cards link to their page (`vcs` and `docs` → `/repository`, `plan` → `/plan`,
   `spec` → `/specs`, `quality` → `/quality`).
7. `pnpm check` in `web/` and `cargo test -p repoview --locked` pass.

## Out of scope

The content of every page (one story each, after this one); live refresh.
