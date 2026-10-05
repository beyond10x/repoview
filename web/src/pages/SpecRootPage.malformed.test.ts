import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import App from '../App.vue'
import { billing } from '../__fixtures__/spec'
import { capturedRoutes, serve } from '../__fixtures__/spec/server'
import { createAppRouter } from '../router'

// story:spec-pages, correction round 1: the server passes `ess` output through unchanged, so the
// page must survive any declaration field being null or absent. The three named cases first, then
// the whole class: every key of every declaration kind (and of a lifecycle, a transition, an
// outcome, a relation, a field and the graph), nulled and deleted in turn.

const MermaidStub = {
  name: 'MermaidView',
  props: { source: { type: String, required: true } },
  template: '<pre data-test="diagram-source">{{ source }}</pre>',
}

type Json = Record<string, unknown>

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

/** The billing page with its IR and graph replaced; Vue render errors collected, not thrown. */
async function mountBilling(ir: unknown, graph: unknown = billing.rawGraph) {
  const routes = capturedRoutes()
  routes[`/api/spec/roots/${billing.root}/ir`] = ir
  routes[`/api/spec/roots/${billing.root}/graph`] = graph
  serve(routes)
  const errors: unknown[] = []
  const router = createAppRouter(createMemoryHistory())
  await router.push(`/specs/${billing.root}`)
  await router.isReady()
  const wrapper = mount(App, {
    global: {
      plugins: [router],
      stubs: { MermaidView: MermaidStub },
      config: {
        errorHandler: (error) => {
          errors.push(error)
        },
      },
    },
  })
  await flushPromises()
  await flushPromises()
  return { wrapper, errors }
}

function sectionCount(wrapper: VueWrapper): number {
  return wrapper.findAll('[data-test="spec-section"]').length
}

function entityCard(wrapper: VueWrapper, name: string) {
  return wrapper.get(`[data-test="entity"][data-entity="${name}"]`)
}

const INVOICE = 'billing.invoice.Invoice'
const CANCEL = 'billing.invoice.CancelInvoice'

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('named cases', () => {
  it('lifecycle: null renders the entity with lifecycle "none" and no diagram', async () => {
    const ir = clone<unknown>(billing.rawIr) as { entities: Record<string, Json> }
    const invoice = ir.entities[INVOICE]
    if (invoice === undefined) throw new Error('fixture')
    invoice.lifecycle = null
    const { wrapper, errors } = await mountBilling(ir)
    expect(errors).toEqual([])
    const card = entityCard(wrapper, INVOICE)
    expect(card.find('[data-test="lifecycle"]').exists()).toBe(false)
    expect(card.get('[data-test="no-lifecycle"]').text()).toContain('none')
    expect(sectionCount(wrapper)).toBe(7)
  })

  it('a command without outcomes renders outcomes "none"', async () => {
    const ir = clone<unknown>(billing.rawIr) as { commands: Record<string, Json> }
    const cancel = ir.commands[CANCEL]
    if (cancel === undefined) throw new Error('fixture')
    delete cancel.outcomes
    const { wrapper, errors } = await mountBilling(ir)
    expect(errors).toEqual([])
    const card = wrapper.get(`[data-test="command"][data-command="${CANCEL}"]`)
    expect(card.findAll('[data-test="outcome"]')).toHaveLength(0)
    expect(card.get('[data-test="no-outcomes"]').text()).toContain('none')
  })

  it('an entity without identity renders identity "none"', async () => {
    const ir = clone<unknown>(billing.rawIr) as { entities: Record<string, Json> }
    const invoice = ir.entities[INVOICE]
    if (invoice === undefined) throw new Error('fixture')
    delete invoice.identity
    const { wrapper, errors } = await mountBilling(ir)
    expect(errors).toEqual([])
    expect(entityCard(wrapper, INVOICE).get('[data-test="identity"]').text()).toContain('none')
  })
})

/** Every path into the billing IR whose value the sweep nulls and deletes. */
function sweepPaths(): string[][] {
  const ir = billing.rawIr as Record<string, Record<string, Json>>
  const paths: string[][] = []
  const keysOf = (value: unknown): string[] =>
    typeof value === 'object' && value !== null ? Object.keys(value) : []
  for (const kind of ['domains', 'entities', 'commands', 'events', 'views', 'components']) {
    paths.push([kind])
    const first = Object.keys(ir[kind] ?? {})[0]
    if (first === undefined) continue
    for (const key of keysOf(ir[kind]?.[first])) paths.push([kind, first, key])
  }
  const invoice = ir.entities?.[INVOICE] as Json
  for (const key of keysOf(invoice.lifecycle)) paths.push(['entities', INVOICE, 'lifecycle', key])
  for (const key of keysOf((invoice.lifecycle as { transitions: Json[] }).transitions[0])) {
    paths.push(['entities', INVOICE, 'lifecycle', 'transitions', '0', key])
  }
  for (const key of keysOf((invoice.fields as Json[])[0])) {
    paths.push(['entities', INVOICE, 'fields', '0', key])
  }
  for (const key of ['identity', 'relations', 'fields', 'lifecycle', 'invariants', 'naming']) {
    paths.push(['entities', INVOICE, key])
  }
  const account = ir.entities?.['billing.invoice.Account'] as Json
  for (const key of keysOf((account.relations as Json[])[0])) {
    paths.push(['entities', 'billing.invoice.Account', 'relations', '0', key])
  }
  const cancel = ir.commands?.[CANCEL] as Json
  for (const key of keysOf((cancel.outcomes as Json[])[0])) {
    paths.push(['commands', CANCEL, 'outcomes', '0', key])
  }
  const subject = (cancel.outcomes as Json[])[0]?.subject
  for (const key of keysOf(subject))
    paths.push(['commands', CANCEL, 'outcomes', '0', 'subject', key])
  return paths
}

function withAt(root: unknown, path: string[], change: 'null' | 'delete'): unknown {
  const copy = clone(root)
  let node = copy as Record<string, unknown>
  for (const key of path.slice(0, -1)) node = node[key] as Record<string, unknown>
  const last = path[path.length - 1] ?? ''
  if (change === 'null') node[last] = null
  else Reflect.deleteProperty(node, last)
  return copy
}

describe('every IR field null or absent: the page still renders, without a render error', () => {
  const paths = sweepPaths()

  it('the sweep covers the declaration kinds and the nested shapes', () => {
    expect(paths.length).toBeGreaterThan(40)
  })

  for (const path of paths) {
    for (const change of ['null', 'delete'] as const) {
      it(`${change} ${path.join('.')}`, async () => {
        const { wrapper, errors } = await mountBilling(withAt(billing.rawIr, path, change))
        expect(errors).toEqual([])
        expect(sectionCount(wrapper)).toBe(7)
      })
    }
  }

  for (const key of Object.keys(billing.rawGraph as Record<string, unknown>)) {
    for (const change of ['null', 'delete'] as const) {
      it(`${change} graph.${key}`, async () => {
        const { wrapper, errors } = await mountBilling(
          billing.rawIr,
          withAt(billing.rawGraph, [key], change),
        )
        expect(errors).toEqual([])
        expect(sectionCount(wrapper)).toBe(7)
      })
    }
  }
})
