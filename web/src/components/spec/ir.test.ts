import { describe, expect, it } from 'vitest'
import { billing, repoview } from '../../__fixtures__/spec'
import type { CommandDecl, Entity } from '../../api/spec'
import { entityAnchor, lifecycleSource, shortName, typeLabel } from './ir'

// story:spec-pages acceptance 3 and 4: the lifecycle diagram is built from the IR.

function entity(ir: typeof billing.ir, name: string): Entity {
  const found = ir.entities[name]
  if (found === undefined) throw new Error(`no entity ${name}`)
  return found
}

/** `state "<label>" as <id>` lines, as a label → id map. */
function stateIds(source: string): Map<string, string> {
  const ids = new Map<string, string>()
  for (const match of source.matchAll(/^\s*state "([^"]*)" as (\w+)$/gm)) {
    ids.set(match[1] ?? '', match[2] ?? '')
  }
  return ids
}

function idOf(ids: Map<string, string>, state: string): string {
  const id = ids.get(state)
  if (id === undefined) throw new Error(`state ${state} is not declared`)
  return id
}

describe('lifecycleSource', () => {
  const invoice = entity(billing.ir, 'billing.invoice.Invoice')
  const lifecycle = invoice.lifecycle
  if (lifecycle === undefined) throw new Error('fixture: Invoice has a lifecycle')
  const transitions = lifecycle.transitions
  const source = lifecycleSource(invoice, billing.ir.commands) ?? ''
  const ids = stateIds(source)

  it('is a stateDiagram-v2 declaring every state', () => {
    expect(source.split('\n')[0]).toBe('stateDiagram-v2')
    for (const state of lifecycle.states) expect(ids.has(state), state).toBe(true)
  })

  it('the fixture has transitions (otherwise this suite tests nothing)', () => {
    expect(transitions.length).toBeGreaterThanOrEqual(3)
  })

  it('names each transition, from each of its states, labelled with the command that moves it', () => {
    for (const transition of transitions) {
      const movers = Object.values(billing.ir.commands)
        .filter((command) =>
          command.outcomes.some(
            (outcome) =>
              outcome.subject?.entity === invoice.name &&
              outcome.subject.transition?.name === transition.name,
          ),
        )
        .map((command) => shortName(command.name))
      expect(movers.length, transition.name).toBeGreaterThan(0)
      for (const from of transition.from) {
        const edge = `${idOf(ids, from)} --> ${idOf(ids, transition.to)}: `
        const line = source.split('\n').find((l) => l.trim().startsWith(edge))
        expect(line, `${from} -> ${transition.to}`).toBeDefined()
        expect(line).toContain(transition.name)
        for (const mover of movers) expect(line).toContain(mover)
      }
    }
  })

  it('starts at the initial state, labelled with the creating command, and ends at each terminal', () => {
    const lines = source.split('\n').map((l) => l.trim())
    expect(lines).toContain(
      `[*] --> ${idOf(ids, lifecycle.initial ?? 'no initial state')}: CreateInvoice`,
    )
    for (const terminal of lifecycle.terminal) {
      expect(lines).toContain(`${idOf(ids, terminal)} --> [*]`)
    }
  })

  it('a one-state lifecycle is its initial and terminal state', () => {
    const project = entity(repoview.ir, 'repoview.project.Project')
    const text = lifecycleSource(project, repoview.ir.commands) ?? ''
    const projectIds = stateIds(text)
    const observed = idOf(projectIds, 'Observed')
    expect(text.split('\n').map((l) => l.trim())).toEqual([
      'stateDiagram-v2',
      `state "Observed" as ${observed}`,
      `[*] --> ${observed}`,
      `${observed} --> [*]`,
    ])
  })

  it('an entity with no lifecycle has no diagram', () => {
    const bare: Entity = { ...invoice, lifecycle: undefined }
    expect(lifecycleSource(bare, {})).toBeNull()
  })

  it('names that could break the diagram are reduced to plain text', () => {
    const hostile: Entity = {
      ...invoice,
      lifecycle: {
        states: ['A"; click A call evil()', 'B\nC'],
        initial: 'A"; click A call evil()',
        terminal: ['B\nC'],
        transitions: [{ name: 'go:%%x', from: ['A"; click A call evil()'], to: 'B\nC' }],
      },
    }
    const commands: Record<string, CommandDecl> = {
      'x.Go': {
        name: 'x.Go<script>',
        domain: 'x',
        input: [],
        response: [],
        outcomes: [
          {
            name: 'went',
            emits: [],
            sets: [],
            subject: {
              entity: invoice.name,
              effect: 'moves',
              transition: { name: 'go:%%x', from: [], to: '' },
            },
          },
        ],
      },
    }
    const text = lifecycleSource(hostile, commands) ?? ''
    const lines = text.split('\n')
    expect(lines).toHaveLength(6)
    // Every line is one of the three statement shapes; a label never closes its quotes, opens a
    // comment (`%%`), ends a statement (`;`) or carries markup.
    const statement =
      /^ {4}(state "[^"]*" as s\d+|\[\*\] --> s\d+(: [^"]*)?|s\d+ --> (s\d+|\[\*\])(: [^"]*)?)$/
    for (const line of lines.slice(1)) {
      expect(line).toMatch(statement)
      const labels = [...line.matchAll(/"([^"]*)"|: (.*)$/g)].map((m) => m[1] ?? m[2] ?? '')
      for (const text of labels) expect(text).not.toMatch(/[;<>%:]/)
    }
    expect(lines.slice(1).filter((line) => line.includes('state "'))).toHaveLength(2)
  })
})

describe('typeLabel', () => {
  it('renders each type reference kind', () => {
    expect(typeLabel({ kind: 'primitive', name: 'string' })).toBe('string')
    expect(typeLabel({ kind: 'declared', name: 'billing.invoice.Money' })).toBe('Money')
    expect(typeLabel({ kind: 'optional', of: { kind: 'primitive', name: 'string' } })).toBe(
      'string?',
    )
    expect(typeLabel({ kind: 'list', of: { kind: 'declared', name: 'a.b.LineItem' } })).toBe(
      'list<LineItem>',
    )
    expect(
      typeLabel({ kind: 'map', key: 'string', value: { kind: 'primitive', name: 'integer' } }),
    ).toBe('map<string, integer>')
    expect(typeLabel({ kind: 'tuple' })).toBe('tuple')
  })
})

describe('names', () => {
  it('shortName is the last segment; entityAnchor is stable per entity', () => {
    expect(shortName('billing.invoice.Invoice')).toBe('Invoice')
    expect(shortName('email-service')).toBe('email-service')
    expect(entityAnchor('billing.invoice.Invoice')).toBe('entity-billing.invoice.Invoice')
  })
})
