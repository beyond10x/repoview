// Captured with ess 0.52.0, stdout only:
//   ess specify validate --path <root> --format json   > <name>.validate.json
//   ess specify compile  --path <root> --format json   > <name>.compile.json
//   ess specify graph    --path <root> --format json   > <name>.graph.json
//   ess specify graph    --path <root> --format mermaid > <name>.graph.mmd
// repoview: this repository's `ess/`; codegate: beyond10x/codegate's `ess/`; billing:
// beyond10x/ess's `examples/billing` (the only one of the three with lifecycle transitions).
// refused.validate.json: validate on a copy of billing with one relation target renamed.
// example-*.compile.json: compile of beyond10x/ess's other examples (gatepass, oracle-fixture,
// revision-pair before/after), used only to check which IR keys the normaliser keeps.
import { normalizeGraph, normalizeIr, type Graph, type Ir, type RootEntry } from '../../api/spec'
import billingIr from './billing.compile.json?raw'
import billingGraph from './billing.graph.json?raw'
import billingMermaid from './billing.graph.mmd?raw'
import billingValidate from './billing.validate.json?raw'
import gatepassIr from './example-gatepass.compile.json?raw'
import oracleIr from './example-oracle-fixture.compile.json?raw'
import revisionAfterIr from './example-revision-pair-after.compile.json?raw'
import revisionBeforeIr from './example-revision-pair-before.compile.json?raw'
import codegateIr from './codegate.compile.json?raw'
import codegateGraph from './codegate.graph.json?raw'
import codegateMermaid from './codegate.graph.mmd?raw'
import codegateValidate from './codegate.validate.json?raw'
import refusedValidate from './refused.validate.json?raw'
import repoviewIr from './repoview.compile.json?raw'
import repoviewGraph from './repoview.graph.json?raw'
import repoviewMermaid from './repoview.graph.mmd?raw'
import repoviewValidate from './repoview.validate.json?raw'

export interface CapturedRoot {
  /** The root's directory as `/api/spec/roots` would name it. */
  root: string
  validate: Record<string, unknown>
  /** The captured `compile` output exactly as ess printed it: what the stand-in server serves. */
  rawIr: unknown
  /** The captured `graph` output exactly as ess printed it. */
  rawGraph: unknown
  /** `rawIr` as the page reads it, for assertions. */
  ir: Ir
  graph: Graph
  mermaid: string
}

function captured(
  root: string,
  validate: string,
  ir: string,
  graph: string,
  mermaid: string,
): CapturedRoot {
  return {
    root,
    validate: JSON.parse(validate) as Record<string, unknown>,
    rawIr: JSON.parse(ir) as unknown,
    rawGraph: JSON.parse(graph) as unknown,
    ir: normalizeIr(JSON.parse(ir)),
    graph: normalizeGraph(JSON.parse(graph)),
    mermaid,
  }
}

export const repoview = captured(
  'ess',
  repoviewValidate,
  repoviewIr,
  repoviewGraph,
  repoviewMermaid,
)
export const codegate = captured(
  'tools/codegate/ess',
  codegateValidate,
  codegateIr,
  codegateGraph,
  codegateMermaid,
)
export const billing = captured(
  'examples/billing',
  billingValidate,
  billingIr,
  billingGraph,
  billingMermaid,
)

export const CAPTURED: readonly CapturedRoot[] = [repoview, codegate, billing]

export const refused = {
  root: 'broken',
  validate: JSON.parse(refusedValidate) as Record<string, unknown>,
}

/** `/api/spec/roots` over every captured root plus the refused one. */
export function rootsAnswer(): RootEntry[] {
  return [
    ...CAPTURED.map((c) => ({ root: c.root, validate: c.validate, ok: true })),
    { root: refused.root, validate: refused.validate, ok: false },
  ]
}

/** Raw `ess specify compile` output of every captured specification: the three roots above and
 * the rest of beyond10x/ess's examples. */
export const EXAMPLE_IRS: readonly { name: string; raw: unknown }[] = [
  ...CAPTURED.map((c) => ({ name: c.root, raw: c.rawIr })),
  ...[
    ['gatepass', gatepassIr],
    ['oracle-fixture', oracleIr],
    ['revision-pair/before', revisionBeforeIr],
    ['revision-pair/after', revisionAfterIr],
  ].map(([name, text]) => ({ name: name ?? '', raw: JSON.parse(text ?? '{}') as unknown })),
]
