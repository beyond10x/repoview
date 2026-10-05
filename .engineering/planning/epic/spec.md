---
format: aep.planning-md/3
id: epic:spec
kind: epic
status: draft
title: ESS specifications browsable per root
summary: Domains, entities with lifecycles, commands, events, views, components and the interaction graph from ess compile and graph.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- depends_on: epic:shell
revision: 1
---
## Outcome

Every ESS specification root in the project is browsable: domains, entities, lifecycles, commands,
events, views, components and the interaction graph.

## Scope

- Source `spec`: find every `ess-inputs.yaml`, else every `system.yaml` whose `format:` starts with
  `ess/`, skipping Git-ignored paths; per root run `ess specify validate --format json`,
  `ess specify compile --format json` and `ess specify graph --format json`.
- Specs page per root: validate result and format; domains; per entity its identity, fields,
  `relations:` as links, and its lifecycle as a state diagram; commands with outcomes; events;
  views; components with what they own, accept and publish; the interaction graph.
- The latest conformance report where one is found beside the root, with its counts.

## Acceptance

- On `beyond10x/codegate` the page lists both roots (`ess/`, `ess-semantic/`) with the domains and
  entities `ess specify compile` returns for each.
- A root that fails validation shows the refusal lines as `ess` printed them, and no IR.

## Depends on

`epic:shell`.
