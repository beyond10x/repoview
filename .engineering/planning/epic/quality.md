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
revision: 2
---
## Outcome

The Quality page shows what the beyond10x Codegate (`github.com/beyond10x/codegate`) says about the
project, with the producing binary and version named, and says plainly what it cannot say yet.

## Scope

- Source `quality`: locate the beyond10x `codegate` on `PATH` by its `--version`; the Go
  `fluxplane/codegate` is skipped and never run for an assessment (operator, 2026-10-05).
- Quality page: producer, the commands Codegate offers, and its assessment once Codegate ships one.
- No score, rating or finding without an assessment.

## Acceptance

- With codegate 0.3.0 installed, Quality names `~/.local/bin/codegate` 0.3.0, lists `evaluate`, and
  states that there is no source assessment yet.
- With only the Go codegate on `PATH`, Quality says the beyond10x codegate was not found and names
  the skipped binary.
- When Codegate adds a source assessment command, a later story renders its output.

## Depends on

`epic:shell`. Stories: `story:quality-page` (run machinery), `story:quality-codegate` (locator and
page).
