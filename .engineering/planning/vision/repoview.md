---
format: aep.planning-md/3
id: vision:repoview
kind: vision
status: draft
title: One view of a project, from the project's own records
summary: Run repoview in a project directory and see its Git state, ESS specifications, AEP plan and Codegate rating in one browser view, read-only.
revision: 1
---
## Intent

Run `repoview` in any project directory and a browser tab shows what matters about that project,
from the project's own records: its version-control state, its ESS specifications, its AEP plan as
a board and as a tree of visions, designs, epics and stories, and its Codegate quality rating. One
command, no configuration, nothing to set up per repository.

Today the same facts are spread over `git`, `aep plan artifact …`, `ess specify …`, `codegate
assess` and a dozen markdown files, each read in a terminal. A person asked "where does this
project stand?" opens five tools. An agent asked the same question runs them one by one and
summarises. repoview puts them on one page and links them to each other: a story to the epic it
decomposes, the design it implements, the ESS entity it introduces and the files its scope names.

## Users

- An engineer opening a repository they have not worked in for a while.
- An operator reviewing what agents planned and built: which stories are proposed, which are
  active, what evidence moved them, what the critics said.
- A stakeholder walked through a design in a browser instead of a terminal.

## Outcomes

1. `repoview` in a directory opens a local browser view of that project within 3 seconds for a
   repository of the size of `ess` (the slowest source may finish later and fills in when done).
2. Every panel names the tool and version that produced it, or says the source is absent, the tool
   is missing, or the tool failed with its diagnostic. A missing source never renders as an empty
   or zero value.
3. The AEP plan is browsable as a board (status columns from the ladders), as a hierarchy
   (vision → design → epic → story → task) and per artifact (rendered body, relations, history,
   evidence, findings).
4. ESS specifications are browsable per root: domains, entities with fields, relations and
   lifecycle diagrams, commands, events, views, components, the interaction graph and the latest
   conformance counts where a report exists.
5. The Codegate rating and scores show per language, with the producer named.

## Scope now

Read-only. repoview writes nothing into the project: no store file, no specification, no Git
object, no cache inside the repository.

## Later

- Editing through the owning tool: lifecycle moves and story approval via `aep plan artifact
  move` and `evidence`, never by writing a store file. repoview never becomes a second writer of
  a store another CLI owns.
- A workspace view over many repositories (for example the beyond10x checkout root), using
  `aep plan workspace` where it answers.
- Live refresh on file change.

## Exclusions

- No hosted or multi-user service. The server binds `127.0.0.1` only.
- No re-implementation of AEP, ESS, Codegate or Git semantics. repoview renders what their CLIs
  answer and links it.
- No replacement for `aep plan serve`, which stays the AEP-specific board with moves.
