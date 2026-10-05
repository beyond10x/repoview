import { describe, expect, it } from 'vitest'
import { EXAMPLE_IRS } from '../__fixtures__/spec'
import { normalizeIr } from './spec'

// story:spec-pages, correction round 2. Every key ess 0.52.0 writes into the compiled IR of the
// captured specifications is either kept by `normalizeIr` (and shown on the page) or listed here
// as deliberately not shown, with the reason. A key ess adds later fails the first case until it
// is sorted into one list or the other; a kept key the normaliser loses fails the second.

type Shape =
  | 'ir'
  | 'domain'
  | 'entity'
  | 'lifecycle'
  | 'transition'
  | 'relation'
  | 'field'
  | 'command'
  | 'outcome'
  | 'condition'
  | 'subject'
  | 'instance'
  | 'set'
  | 'setValue'
  | 'event'
  | 'view'
  | 'component'

const SHOWN: Record<Shape, string[]> = {
  ir: ['system', 'version', 'domains', 'entities', 'commands', 'events', 'views', 'components'],
  domain: [
    'name',
    'naming',
    'types',
    'entities',
    'commands',
    'events',
    'errors',
    'views',
    'actors',
  ],
  entity: [
    'name',
    'domain',
    'identity',
    'fields',
    'lifecycle',
    'invariants',
    'relations',
    'naming',
  ],
  lifecycle: ['states', 'initial', 'terminal', 'transitions'],
  transition: ['name', 'from', 'to'],
  relation: ['name', 'kind', 'target', 'cardinality', 'via'],
  field: ['name', 'type_ref', 'naming'],
  command: ['name', 'domain', 'input', 'response', 'outcomes', 'naming'],
  outcome: ['name', 'condition', 'subject', 'emits', 'error', 'summary', 'sets', 'returns'],
  condition: ['kind', 'cause', 'predicate'],
  subject: ['entity', 'effect', 'transition', 'instance'],
  instance: ['from', 'event', 'field'],
  set: ['target', 'value'],
  setValue: ['kind', 'field', 'value'],
  event: ['name', 'domain', 'fields', 'naming'],
  view: ['name', 'domain', 'source', 'fields', 'consistency', 'filter', 'order_by', 'naming'],
  component: ['name', 'owns', 'accepts', 'publishes', 'reached_by', 'naming'],
}

/** Keys the page does not show, and why. */
const NOT_SHOWN: Partial<Record<Shape, Record<string, string>>> = {
  ir: {
    naming: 'system-level naming; the heading shows the system name',
    summary: 'system summary; not part of the story',
    types: 'types have no section of their own; fields show their type names',
    conversions: 'not part of the story',
    errors: 'shown as names on the outcomes that answer with them',
    actors: 'shown in the interaction graph',
    bindings: 'realization detail, not part of the story',
    workloads: 'realization detail, not part of the story',
  },
  entity: { state_type: 'the generated name of the state enum; the lifecycle shows the states' },
  outcome: {
    payload: 'the event payload mapping; the outcome names the events it emits',
    test_strategy: 'conformance detail; conformance reports are a later story',
  },
  set: { target_type: 'the type of the target field, shown on the entity' },
  setValue: { type_ref: 'the type of the input field, shown on the command input' },
  view: { assertion_style: 'conformance detail; conformance reports are a later story' },
}

type Raw = Record<string, unknown>

function obj(value: unknown): Raw | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Raw)
    : null
}

function items(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function values(value: unknown): [string, unknown][] {
  return Object.entries(obj(value) ?? {})
}

/** Every (shape, raw object, normalised counterpart) pair in one compiled IR. */
function pairs(raw: unknown): [Shape, string, Raw, unknown][] {
  const ir = normalizeIr(raw) as unknown as Raw
  const out: [Shape, string, Raw, unknown][] = []
  const add = (shape: Shape, where: string, r: unknown, n: unknown): void => {
    const rawObj = obj(r)
    if (rawObj !== null) out.push([shape, where, rawObj, n])
  }
  const at = (n: unknown, key: string | number): unknown =>
    Array.isArray(n) ? (n as unknown[])[key as number] : obj(n)?.[key]
  const fieldsOf = (where: string, r: unknown, n: unknown): void => {
    items(r).forEach((f, i) => {
      add('field', `${where}[${String(i)}]`, f, at(n, i))
    })
  }
  const rawIr = obj(raw) ?? {}
  add('ir', '', rawIr, ir)
  for (const [k, d] of values(rawIr.domains)) add('domain', k, d, at(ir.domains, k))
  for (const [k, e] of values(rawIr.entities)) {
    const ne = at(ir.entities, k)
    const re = obj(e) ?? {}
    add('entity', k, re, ne)
    add('field', `${k}.identity`, re.identity, at(ne, 'identity'))
    fieldsOf(`${k}.fields`, re.fields, at(ne, 'fields'))
    add('lifecycle', `${k}.lifecycle`, re.lifecycle, at(ne, 'lifecycle'))
    const rl = obj(re.lifecycle) ?? {}
    items(rl.transitions).forEach((t, i) => {
      add(
        'transition',
        `${k}.transitions[${String(i)}]`,
        t,
        at(at(at(ne, 'lifecycle'), 'transitions'), i),
      )
    })
    items(re.relations).forEach((r, i) => {
      add('relation', `${k}.relations[${String(i)}]`, r, at(at(ne, 'relations'), i))
    })
  }
  for (const [k, c] of values(rawIr.commands)) {
    const nc = at(ir.commands, k)
    const rc = obj(c) ?? {}
    add('command', k, rc, nc)
    fieldsOf(`${k}.input`, rc.input, at(nc, 'input'))
    fieldsOf(`${k}.response`, rc.response, at(nc, 'response'))
    items(rc.outcomes).forEach((o, i) => {
      const where = `${k}.outcomes[${String(i)}]`
      const no = at(at(nc, 'outcomes'), i)
      const ro = obj(o) ?? {}
      add('outcome', where, ro, no)
      add('condition', `${where}.condition`, ro.condition, at(no, 'condition'))
      add('subject', `${where}.subject`, ro.subject, at(no, 'subject'))
      const rs = obj(ro.subject) ?? {}
      add(
        'transition',
        `${where}.subject.transition`,
        rs.transition,
        at(at(no, 'subject'), 'transition'),
      )
      add('instance', `${where}.subject.instance`, rs.instance, at(at(no, 'subject'), 'instance'))
      items(ro.sets).forEach((s, j) => {
        const ns = at(at(no, 'sets'), j)
        add('set', `${where}.sets[${String(j)}]`, s, ns)
        add('setValue', `${where}.sets[${String(j)}].value`, obj(s)?.value, at(ns, 'value'))
      })
    })
  }
  for (const [k, e] of values(rawIr.events)) {
    add('event', k, e, at(ir.events, k))
    fieldsOf(`${k}.fields`, obj(e)?.fields, at(at(ir.events, k), 'fields'))
  }
  for (const [k, v] of values(rawIr.views)) {
    add('view', k, v, at(ir.views, k))
    fieldsOf(`${k}.fields`, obj(v)?.fields, at(at(ir.views, k), 'fields'))
  }
  for (const [k, c] of values(rawIr.components)) add('component', k, c, at(ir.components, k))
  return out
}

describe('the IR keys ess writes, against what the page reads', () => {
  it('the captured specifications are the five ess examples and repoview and codegate', () => {
    expect(EXAMPLE_IRS.map((e) => e.name)).toHaveLength(7)
    expect(EXAMPLE_IRS.reduce((n, e) => n + pairs(e.raw).length, 0)).toBeGreaterThan(300)
  })

  it('every key is either shown or listed as not shown', () => {
    const unsorted: string[] = []
    for (const { name, raw } of EXAMPLE_IRS) {
      for (const [shape, where, rawObj] of pairs(raw)) {
        for (const key of Object.keys(rawObj)) {
          if (SHOWN[shape].includes(key) || NOT_SHOWN[shape]?.[key] !== undefined) continue
          unsorted.push(`${name}: ${shape} ${where}: ${key}`)
        }
      }
    }
    expect(unsorted).toEqual([])
  })

  it('every shown key ess gave a value survives normalising', () => {
    const lost: string[] = []
    for (const { name, raw } of EXAMPLE_IRS) {
      for (const [shape, where, rawObj, normalised] of pairs(raw)) {
        const kept = obj(normalised)
        for (const key of SHOWN[shape]) {
          const given = rawObj[key]
          if (given === undefined || given === null) continue
          if (kept?.[key] === undefined) lost.push(`${name}: ${shape} ${where}: ${key}`)
        }
      }
    }
    expect(lost).toEqual([])
  })
})
