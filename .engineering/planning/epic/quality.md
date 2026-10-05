---
format: aep.planning-md/3
id: epic:quality
kind: epic
status: draft
title: Codegate rating per language
summary: Rating, scores and findings from codegate assess, with unsupported languages shown as not assessed.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- depends_on: epic:shell
revision: 1
---
## Outcome

The Codegate rating, scores and finding counts show per language the project contains, with the
producing binary and version named.

## Scope

- Source `quality`: detect languages from manifests (`Cargo.toml`, `go.mod`, `package.json`,
  `*.md`); run `codegate --root . --language <l> --format json assess --gate all` for each language
  the installed codegate reports in `codegate capabilities`; run in the background, the panel fills
  in when done.
- Quality page: `rating`, `scores`, `summary`, `finding_counts`, `top_findings`, the producer.
- The rating on Overview.
- A language the installed codegate does not support shows "not assessed" with the reason, never a
  score.

## Acceptance

- On a Go repository the rating matches `codegate assess` run by hand.
- On a Rust-only repository with the current `fluxplane/codegate`, Quality shows Rust "not
  assessed: codegate does not support rust" and Markdown assessed.

## Depends on

`epic:shell`. The assessment format follows `beyond10x/codegate` once it ships scoring
(`architecture-design:repoview`, section Codegate).
