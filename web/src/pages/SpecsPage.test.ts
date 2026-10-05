import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, type Router } from 'vue-router'
import App from '../App.vue'
import { CAPTURED, refused } from '../__fixtures__/spec'
import { capturedRoutes, json, serve } from '../__fixtures__/spec/server'
import { createAppRouter } from '../router'

// story:spec-pages acceptance 2: one row per root with system, version, format and validate result.

async function mountSpecs(): Promise<{ wrapper: VueWrapper; router: Router }> {
  const router = createAppRouter(createMemoryHistory())
  await router.push('/specs')
  await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } })
  await flushPromises()
  return { wrapper, router }
}

function row(wrapper: VueWrapper, root: string) {
  const found = wrapper
    .findAll('[data-test="spec-root-row"]')
    .find((r) => r.attributes('data-root') === root)
  if (found === undefined) throw new Error(`no row for ${root}`)
  return found
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('Specs page', () => {
  it('keeps its title', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountSpecs()
    expect(wrapper.get('main h1').text()).toBe('Specs')
  })

  it('has one row per root', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountSpecs()
    expect(
      wrapper.findAll('[data-test="spec-root-row"]').map((r) => r.attributes('data-root')),
    ).toEqual([...CAPTURED.map((c) => c.root), refused.root])
  })

  it('a valid root shows its system name, version, format and "valid"', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountSpecs()
    for (const c of CAPTURED) {
      const r = row(wrapper, c.root)
      expect(r.get('[data-test="system"]').text()).toBe(String(c.validate.system))
      expect(r.get('[data-test="version"]').text()).toBe(String(c.validate.version))
      // ess 0.52.0's validate JSON carries no format; the page says so instead of guessing.
      expect(r.get('[data-test="format"]').text()).toBe('not reported by ess')
      expect(r.get('[data-test="validate"]').text()).toBe('valid')
      expect(r.attributes('data-ok')).toBe('true')
    }
  })

  it('a format the tool does report is shown as reported', async () => {
    const routes = capturedRoutes()
    routes['/api/spec/roots'] = [
      {
        root: 'ess',
        ok: true,
        validate: { valid: true, system: 's', version: 'v1', format: 'ess/20' },
      },
    ]
    serve(routes)
    const { wrapper } = await mountSpecs()
    expect(row(wrapper, 'ess').get('[data-test="format"]').text()).toBe('ess/20')
  })

  it('a refused root shows the refusal lines verbatim', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountSpecs()
    const r = row(wrapper, refused.root)
    expect(r.attributes('data-ok')).toBe('false')
    const lines = r.findAll('[data-test="refusal-line"]').map((line) => line.text())
    expect(lines).toEqual(refused.validate.problems)
    expect(r.get('[data-test="system"]').text()).toBe('not reported')
  })

  it('each row links to its root page; the project root `.` links to /specs/~ (coordinator addition)', async () => {
    const routes = capturedRoutes()
    routes['/api/spec/roots'] = [
      { root: '.', ok: true, validate: { valid: true, system: 'top', version: 'v1' } },
      { root: 'crates/a/ess', ok: true, validate: { valid: true, system: 'a', version: 'v1' } },
    ]
    serve(routes)
    const { wrapper } = await mountSpecs()
    expect(row(wrapper, '.').get('a').attributes('href')).toBe('/specs/~')
    expect(row(wrapper, 'crates/a/ess').get('a').attributes('href')).toBe('/specs/crates/a/ess')
  })

  it('names the ess that answered, from the snapshot', async () => {
    serve(capturedRoutes())
    const { wrapper } = await mountSpecs()
    expect(wrapper.get('[data-test="producer"]').text()).toContain('ess 0.52.0')
  })

  it('no roots is said, never an empty table', async () => {
    const routes = capturedRoutes()
    routes['/api/spec/roots'] = []
    serve(routes)
    const { wrapper } = await mountSpecs()
    expect(wrapper.find('[data-test="spec-root-row"]').exists()).toBe(false)
    expect(wrapper.get('[data-test="spec-empty"]').text()).toContain('no ESS specification root')
  })

  it('a missing ess (503) is an error notice with the status', async () => {
    const routes = capturedRoutes()
    routes['/api/spec/roots'] = json({ tool: 'ess', exit: null, stderr: 'ess not found' }, 503)
    serve(routes)
    const { wrapper } = await mountSpecs()
    expect(wrapper.get('[role="alert"]').text()).toContain('503')
  })
})
