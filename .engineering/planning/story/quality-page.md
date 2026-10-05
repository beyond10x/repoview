---
format: aep.planning-md/3
id: story:quality-page
kind: story
status: implemented
title: Codegate rating per language
summary: codegate capabilities and assess behind /api/quality, run in the background; unsupported languages shown as not assessed.
relations:
- decomposes: epic:quality
- serves: vision:repoview
- depends_on: story:page-frame
scope:
- confidence: cited
  path: crates/repoview-sources/src/quality.rs
- confidence: cited
  path: crates/repoview/src/api/quality.rs
- confidence: cited
  path: crates/repoview/tests/api_quality.rs
- confidence: cited
  path: web/src/api/quality.ts
- confidence: cited
  path: web/src/components/quality/
- confidence: cited
  path: web/src/pages/QualityPage.vue
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-05T13:40:01Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-05T15:16:16Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Story

As an engineer, I see the project's Codegate rating and scores per language, with the producing
binary named, and a plain "not assessed" with the reason for each language Codegate does not
support.

## Acceptance

1. Languages are detected from the project root: `go.mod` → go, `Cargo.toml` → rust,
   `package.json` → typescript, `pom.xml` or `build.gradle*` → java, any tracked `*.md` → markdown.
   Supported languages are read from `codegate capabilities` (the `language` field of each entry),
   never from a list in repoview.
2. API `GET /api/quality` (token-guarded) answers
   `{ "tool": "codegate", "tool_path": "...", "languages": [ { "language": "go", "status": "assessed" | "not-assessed" | "failed" | "running", "reason": "...", "assessment": <codegate --root <root> --language <l> --format json assess --gate all, passed through> } ] }`.
   Assessments run in the background on first request, one per supported language, with a 120 s
   timeout each; until one finishes its status is `running`. A language codegate does not support
   is `not-assessed` with reason `codegate does not support <language>`. No codegate on `PATH`:
   503 `{ "tool": "codegate", "exit": null, "stderr": "codegate not found on PATH" }`.
3. Quality page (`/quality`): per language a card with `rating`, `scores` as bars from 0 to
   `score_max`, `summary`, `finding_counts`, and `top_findings` (title, location); `running` shows a
   spinner and polls every 2 s; `not-assessed` shows the reason; `failed` shows stderr. No card ever
   shows a score for a language that was not assessed (vitest).
4. Rust tests with a stub `codegate` cover capabilities parsing, a supported and an unsupported
   language, a failing assess and a timeout. Vitest uses a fixture captured from
   `codegate --root . --language markdown --format json assess --gate all` on this repository.
5. Gates: `cargo test -p repoview -p repoview-sources --locked`, clippy `-D warnings`,
   `cd web && pnpm check`.

## Scope

Owns `crates/repoview-sources/src/quality.rs`, `crates/repoview/src/api/quality.rs`,
`crates/repoview/tests/api_quality.rs`, `web/src/pages/QualityPage.vue`, `web/src/api/quality.ts`,
`web/src/components/quality/`, `web/src/__fixtures__/quality/` and their tests.

## Out of scope

Choosing between Codegate implementations; history of ratings over time.
