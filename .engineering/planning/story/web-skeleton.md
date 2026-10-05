---
format: aep.planning-md/3
id: story:web-skeleton
kind: story
status: implemented
title: 'Vue app: Overview page over the snapshot'
summary: Vue 3 + Vite SPA rendering one card per source with availability and producer; token moved from URL to sessionStorage.
relations:
- decomposes: epic:shell
- serves: vision:repoview
scope:
- confidence: cited
  path: web/
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T12:37:44Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-05T12:37:44Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-05T13:24:12Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":5,"verification":1}}}
---
## Story

As an engineer who opened the URL `repoview open` printed, I see an Overview page: the project name
and root, and one card per source showing its availability, the tool and version that answered,
and its summary, with absent, missing and failed sources visibly distinct.

## Acceptance

1. `pnpm install --frozen-lockfile && pnpm check` in `web/` exits 0 (typecheck, lint, `vitest run`,
   `vite build`), and the red `vitest` run before the implementation is in the implementor's report.
2. `pnpm build` writes `web/dist/index.html` and hashed assets under `web/dist/assets/`.
3. Vitest cases, against fixture snapshots in `web/src/__fixtures__/`:
   - a snapshot with all five sources `Present` renders five cards; each names its `tool` and
     `tool_version`, or "no external tool" where `tool` is `null` (`docs` has no tool, per the wire
     contract);
   - `Absent` renders "absent", `ToolMissing` renders "tool missing: <tool>", `Failed` renders
     "failed" plus the `diagnostic` text; none of the three renders an empty card or a `0`;
   - a 403 from `/api/snapshot` renders "token rejected — reopen the URL repoview printed";
   - the `vcs` card shows `summary.branch` and the first 7 characters of `summary.head`.
4. Token handling per `story:server-skeleton` § Wire contract, "Token transport": on first load the
   `token` query parameter is moved into `sessionStorage` and removed from the address bar with
   `history.replaceState`; a vitest case asserts both, and that the next `/api/snapshot` fetch
   carries header `X-Repoview-Token` with that value.
5. `vite.config.ts` proxies `/api` to `http://127.0.0.1:7480` in dev with `changeOrigin: true`, so the
   proxied request carries `Host: 127.0.0.1:7480` and passes the server's Host check.
6. Static mode: when `index.html` carries `<meta name="repoview-mode" content="static">`, the client
   reads `./data/snapshot.json` (relative to the page) instead of `/api/snapshot` and sends no token;
   a vitest case covers both modes.
7. `vite.config.ts` sets `base: "./"`, and the built `web/dist/index.html` references its scripts
   and styles as `./assets/…` (checked by a vitest case or a build assertion).

## Wire contract

Exactly the snapshot shape and token transport in `story:server-skeleton` § Wire contract. Type it once in
`web/src/api/snapshot.ts`; the fixtures conform to it.

## Design notes

- Vue 3, Vite, TypeScript (strict), vue-router (web history when served by `repoview`, hash history in static mode so deep links work on a static host; routes `/` = Overview, and a
  catch-all "not found" page), Vitest + @vue/test-utils + jsdom, ESLint + Prettier.
- Pin exact versions; commit `pnpm-lock.yaml`; `packageManager` field set.
- No state library; one `useSnapshot()` composable over `fetch`.
- Layout: a top bar (project name, root), a left nav with Overview only (later epics add Plan,
  Specs, Quality, Repository), the main area. Plain CSS with custom properties; light and dark via
  `prefers-color-scheme`. No component library.
- No markdown rendering in this story (later epics add markdown-it + DOMPurify).
- No Node scripts besides the `package.json` script entries calling vite, vitest, vue-tsc, eslint
  and prettier.

## Out of scope

The Rust server (`story:server-skeleton`); Plan, Specs, Quality and Repository pages; live refresh.

