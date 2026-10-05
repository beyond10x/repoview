---
format: aep.planning-md/3
id: epic:plan
kind: epic
status: draft
title: AEP plan as board, tree and artifact pages
summary: Board columns from aep, a vision-to-task hierarchy, and every artifact rendered with its relations and history.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- depends_on: epic:shell
revision: 1
---
## Outcome

The AEP plan of the project is browsable in three pages: a board, a hierarchy and one artifact.

## Scope

- Source `plan`: detect `.engineering/project.yaml`; read `aep plan artifact list`, `board`,
  `graph`, `validate` with `--format json`; per artifact `show`, `history`, `explain`.
- Board page: columns and their descriptions from `board --format json`, never a local status
  table; filter by kind, tag and text.
- Tree page: vision → designs → epics → stories → tasks from `serves`, `designs`, `decomposes`
  and `implements` edges; an artifact with no parent edge is listed under "unattached".
- Artifact page: body rendered from markdown (sanitised), relations as links, scope, history,
  explain, findings and outcomes.
- The validate result on Overview: valid, or the defect count with a link to the full output.
- A link to `aep plan serve` for moves.

## Acceptance

- On this repository's own store, the board shows the same artifacts per status as
  `aep plan artifact board`.
- Every relation target on an artifact page opens that artifact's page.
- A store that fails `validate` shows each defect line as `aep` printed it.

## Depends on

`epic:shell`.
