import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, type Router } from 'vue-router'
import App from '../App.vue'
import { billing, CAPTURED, codegate, refused, repoview } from '../__fixtures__/spec'
import { capturedRoutes, fetched, json, serve } from '../__fixtures__/spec/server'
import { entityAnchor, shortName } from '../components/spec/ir'
import MermaidView from '../components/MermaidView.vue'
import { createAppRouter } from '../router'

// story:spec-pages acceptances 3 and 4, against the captured ess output. Mermaid needs a real
// layout engine, so MermaidView is stubbed and each diagram is checked as the source the page hands
// it; MermaidView.test.ts covers the handoff to mermaid.

const MermaidStub = {
  name: 'MermaidView',
  props: { source: { type: String, required: true } },
  template: '<pre data-test="diagram-source">{{ source }}</pre>',
}

const SECTIONS = [
  'Domains',
  'Entities',
  'Commands',
  'Events',
  'Views',
  'Components',
  'Interaction graph',
]

async function mountAt(path: string): Promise<{ wrapper: VueWrapper; router: Router }> {
  const router = createAppRouter(createMemoryHistory())
  await router.push(path)
  await router.isReady()
  const wrapper = mount(App, {
    global: { plugins: [router], stubs: { MermaidView: MermaidStub } },
    attachTo: document.body,
  })
  await flushPromises()
  await flushPromises()
  return { wrapper, router }
}

function sectionTitles(wrapper: VueWrapper): string[] {
  return wrapper.findAll('[data-test="spec-section"] > h2').map((h) => h.text())
}

/** The Mermaid source of the diagram inside `selector`. */
function diagramIn(wrapper: VueWrapper, selector: string): string {
  const container = wrapper.get(selector).element
  const view = wrapper
    .findAllComponents(MermaidView)
    .find((candidate) => container.contains(candidate.element))
  if (view === undefined) throw new Error(`no diagram in ${selector}`)
  return view.props('source') as string
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
})

describe('Spec root page over captured ess output', () => {
  for (const c of CAPTURED) {
    describe(`${c.root} (${c.ir.system})`, () => {
      it('keeps its title and names the root and system', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        expect(wrapper.get('main h1').text()).toBe('Spec root')
        const heading = wrapper.get('[data-test="spec-root-heading"]').text()
        expect(heading).toContain(c.ir.system)
        expect(heading).toContain(c.root)
      })

      it('has the seven sections in order', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        expect(sectionTitles(wrapper)).toEqual(SECTIONS)
      })

      it('every entity name in the IR appears on the page, each with its anchor', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        const text = wrapper.text()
        for (const name of Object.keys(c.ir.entities)) {
          expect(text, name).toContain(name)
          expect(document.getElementById(entityAnchor(name)), name).not.toBeNull()
        }
      })

      it('every domain, command, event, view and component appears', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        const text = wrapper.text()
        for (const group of [c.ir.domains, c.ir.commands, c.ir.events, c.ir.views]) {
          for (const name of Object.keys(group)) expect(text, name).toContain(name)
        }
        for (const name of Object.keys(c.ir.components)) expect(text, name).toContain(name)
      })

      it('each entity shows its identity and every field with its type', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        for (const entity of Object.values(c.ir.entities)) {
          const card = wrapper.get(`[data-test="entity"][data-entity="${entity.name}"]`)
          expect(card.get('[data-test="identity"]').text()).toContain(
            entity.identity?.name ?? 'fixture: no identity',
          )
          const rows = card.findAll('[data-test="field"]').map((r) => r.attributes('data-field'))
          expect(rows).toEqual(entity.fields.map((f) => f.name))
          for (const r of card.findAll('[data-test="field"]')) {
            expect(r.get('[data-test="type"]').text().trim()).not.toBe('')
          }
        }
      })

      it('the interaction graph is the mermaid route text, or says it is empty', async () => {
        serve(capturedRoutes())
        const { wrapper } = await mountAt(`/specs/${c.root}`)
        const section = '[data-test="spec-section"][data-section="graph"]'
        if (c.graph.nodes.length === 0) {
          expect(wrapper.get(section).text()).toContain('no interactions declared')
          expect(wrapper.get(section).findComponent(MermaidView).exists()).toBe(false)
        } else {
          expect(diagramIn(wrapper, section)).toBe(c.mermaid)
        }
      })
    })
  }
})

describe('lifecycles and relations (billing, the fixture with transitions)', () => {
  it('the lifecycle diagram of an entity with transitions names each one', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt(`/specs/${billing.root}`)
    const invoice = billing.ir.entities['billing.invoice.Invoice']
    const transitions = invoice?.lifecycle?.transitions ?? []
    expect(transitions.length).toBeGreaterThan(0)
    const source = diagramIn(
      wrapper,
      '[data-test="entity"][data-entity="billing.invoice.Invoice"] [data-test="lifecycle"]',
    )
    expect(source.startsWith('stateDiagram-v2')).toBe(true)
    for (const transition of transitions) expect(source).toContain(transition.name)
    for (const command of ['IssueInvoice', 'PayInvoice', 'CancelInvoice', 'CreateInvoice']) {
      expect(source).toContain(command)
    }
  })

  it('every entity with a lifecycle gets a diagram', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt(`/specs/${billing.root}`)
    for (const entity of Object.values(billing.ir.entities)) {
      if (entity.lifecycle === undefined) continue
      const source = diagramIn(
        wrapper,
        `[data-test="entity"][data-entity="${entity.name}"] [data-test="lifecycle"]`,
      )
      for (const state of entity.lifecycle.states) expect(source).toContain(`"${state}"`)
    }
  })

  it('relations link to the target entity anchor, which exists on the page', async () => {
    serve(capturedRoutes())
    const { wrapper, router } = await mountAt(`/specs/${billing.root}`)
    const account = wrapper.get('[data-test="entity"][data-entity="billing.invoice.Account"]')
    const relation = account.get('[data-test="relation"][data-relation="invoices"]')
    const link = relation.get('a')
    const anchor = entityAnchor('billing.invoice.Invoice')
    expect(link.attributes('href')).toBe(`/specs/${billing.root}#${anchor}`)
    expect(link.text()).toBe(shortName('billing.invoice.Invoice'))
    expect(relation.text()).toContain('owns')
    expect(relation.text()).toContain('many')
    expect(document.getElementById(anchor)).not.toBeNull()
    await link.trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.hash).toBe(`#${anchor}`)
    expect(router.currentRoute.value.params.root).toBe(billing.root)
  })

  it('field types read as types: lists, optionals and declared names', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt(`/specs/${billing.root}`)
    const text = wrapper.get('[data-test="spec-section"][data-section="entities"]').text()
    expect(text).toContain('list<LineItem>')
  })

  it('a view shows its filter and its order (correction round 2)', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt(`/specs/${billing.root}`)
    const outstanding = wrapper.get(
      '[data-test="view"][data-view="billing.invoice.OutstandingInvoices"]',
    )
    expect(outstanding.get('[data-test="view-filter"]').text()).toBe('state == Issued')
    expect(outstanding.get('[data-test="view-order"]').text()).toBe('issued_at desc')
    // A view ess gives neither for shows neither, rather than an empty line.
    const byId = wrapper.get('[data-test="view"][data-view="billing.invoice.InvoiceById"]')
    expect(byId.find('[data-test="view-filter"]').exists()).toBe(false)
    expect(byId.find('[data-test="view-order"]').exists()).toBe(false)
  })
})

describe('roots that are not shown as IR', () => {
  it('a root that fails validation shows the refusal and no IR sections, and fetches no IR', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt(`/specs/${refused.root}`)
    const refusal = wrapper.get('[data-test="refusal"]')
    expect(refusal.findAll('[data-test="refusal-line"]').map((l) => l.text())).toEqual(
      refused.validate.problems,
    )
    expect(wrapper.find('[data-test="spec-section"]').exists()).toBe(false)
    expect(fetched().filter((path) => path.startsWith(`/api/spec/roots/${refused.root}`))).toEqual(
      [],
    )
  })

  it('a root the server did not detect says so', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountAt('/specs/no/such/root')
    expect(wrapper.text()).toContain('not a detected specification root')
    expect(wrapper.find('[data-test="spec-section"]').exists()).toBe(false)
  })

  it('the project root `.` is /specs/~ and reads the ~ routes (coordinator addition)', async () => {
    const routes = capturedRoutes()
    routes['/api/spec/roots'] = [{ root: '.', ok: true, validate: repoview.validate }]
    routes['/api/spec/roots/~/ir'] = repoview.rawIr
    routes['/api/spec/roots/~/graph'] = repoview.rawGraph
    routes['/api/spec/roots/~/mermaid'] = { mermaid: repoview.mermaid }
    serve(routes)
    const { wrapper } = await mountAt('/specs/~')
    expect(fetched()).toEqual(
      expect.arrayContaining([
        '/api/spec/roots/~/ir',
        '/api/spec/roots/~/graph',
        '/api/spec/roots/~/mermaid',
      ]),
    )
    expect(fetched().some((path) => path.split('/').includes('.'))).toBe(false)
    expect(sectionTitles(wrapper)).toEqual(SECTIONS)
    expect(wrapper.get('[data-test="spec-root-heading"]').text()).toContain('.')
  })

  it('an IR route failure is an error notice with the status, not a blank page', async () => {
    const routes = capturedRoutes()
    routes[`/api/spec/roots/${codegate.root}/ir`] = json(
      { tool: 'ess', exit: 4, stderr: 'boom' },
      502,
    )
    serve(routes)
    const { wrapper } = await mountAt(`/specs/${codegate.root}`)
    expect(wrapper.get('[role="alert"]').text()).toContain('502')
  })
})
