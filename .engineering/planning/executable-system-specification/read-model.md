---
format: aep.planning-md/3
id: executable-system-specification:read-model
kind: executable-system-specification
status: draft
title: 'repoview read model: Project and its Sources'
summary: Project owns many Sources, each with a kind, an availability and the producing tool.
relations:
- derived_from: architecture-design:repoview
revision: 1
---
## Contract

`ess/system.yaml` (`format: ess/20`) and `ess/domains/project.yaml` declare the read model's own
nouns: `repoview.project.Project` (identity `root`) owning many `repoview.project.Source` (via
`project_root`), with `SourceKind` (`Vcs`, `Planning`, `Specification`, `Quality`, `Documents`) and
`Availability` (`Present`, `Absent`, `ToolMissing`, `Failed`). `ess/ess-inputs.yaml` lists both
files and no scenarios.

Facts owned by another system are not re-modelled: AEP artifacts, ESS IR, Codegate assessments and
Git objects arrive as their tools' own JSON.

## Open

Two `UNMAPPED:` markers in `ess/domains/project.yaml`: how a Present source's payload is typed
(a named external format or `Json`), and the views the HTTP API answers. Whether `Project` and
`Source` are entities or immutable structs is open question 1 of `architecture-design:repoview`.

## Evidence observed in planning

`ess specify validate --path ess` (ess 0.52.0, 2026-10-05):

    repoview v1 — 2 file(s), valid
