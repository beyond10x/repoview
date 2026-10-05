---
format: aep.planning-md/3
id: story:spec-pages
kind: story
status: implemented
title: ESS roots, entities with lifecycle diagrams, interaction graph
summary: ess validate, compile and graph behind /api/spec/*; per-root page with entities, lifecycles and the graph in Mermaid.
relations:
- decomposes: epic:spec
- serves: vision:repoview
- depends_on: story:page-frame
scope:
- confidence: cited
  path: crates/repoview-sources/src/lib.rs
- confidence: cited
  path: crates/repoview-sources/src/spec.rs
- confidence: cited
  path: crates/repoview/src/api/spec.rs
- confidence: cited
  path: crates/repoview/tests/api_spec.rs
- confidence: cited
  path: web/src/api/spec.ts
- confidence: cited
  path: web/src/components/spec/
- confidence: cited
  path: web/src/pages/SpecRootPage.vue
- confidence: cited
  path: web/src/pages/SpecsPage.vue
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:16Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Story

As an engineer, I browse every ESS specification root in the project: its validation result, its
domains and entities with fields, relations and lifecycle diagrams, its commands, events, views and
components, and the interaction graph.

## Acceptance

1. API (token-guarded `GET`):
   | route | answer |
   |---|---|
   | `/api/spec/roots` | `[{ "root": "<relative dir>", "validate": <ess specify validate --path <root> --format json> , "ok": bool }]` for every root the `spec` source detected |
   | `/api/spec/roots/{root}/ir` | `ess specify compile --path <root> --format json`, passed through |
   | `/api/spec/roots/{root}/graph` | `ess specify graph --path <root> --format json`, passed through |
   | `/api/spec/roots/{root}/mermaid` | `ess specify graph --path <root> --format mermaid` as `{ "mermaid": "<text>" }` |
   `{root}` (URL-encoded, may contain `/`) must be one of the detected roots exactly, else 404 and no
   process starts. Tool failure is 502, missing tool 503, both `{ "tool": "ess", "exit", "stderr" }`.
   Rust tests use a stub `ess`, including a `{root}` of `../x`, an absolute path and an undetected
   directory.
2. Specs page (`/specs`): one row per root with its system name, version, format and validate result
   (valid, or the refusal lines verbatim).
3. Root page (`/specs/<root>`): sections Domains, Entities, Commands, Events, Views, Components,
   Interaction graph. Each entity shows identity, fields with types, `relations:` entries as links to
   the target entity's anchor, and its lifecycle as a Mermaid `stateDiagram-v2` built from the IR
   (initial state, transitions labelled with the command, terminal states). The interaction graph
   uses the `mermaid` route through `MermaidView`. A root that fails validation shows the refusal
   and no IR sections.
4. Vitest uses fixtures captured from `ess specify compile --path ess --format json` and
   `ess specify graph --path ess --format json` on this repository and on `beyond10x/codegate`'s
   `ess/` (copied into `web/src/__fixtures__/spec/`), and checks that every entity name in the IR
   appears on the page and that the lifecycle source for an entity with transitions names each one.
5. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Scope

Owns `crates/repoview-sources/src/spec.rs`, `crates/repoview/src/api/spec.rs`,
`crates/repoview/tests/api_spec.rs`, `web/src/pages/SpecsPage.vue`, `web/src/pages/SpecRootPage.vue`,
`web/src/api/spec.ts`, `web/src/components/spec/`, `web/src/__fixtures__/spec/` and their tests.

## Out of scope

Conformance reports (a later story); editing; `ess` versions other than the one on `PATH`.
