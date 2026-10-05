import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import App from './App.vue'
import { FAILED_DIAGNOSTIC, VCS_HEAD, allPresent, degraded } from './__fixtures__/snapshots'
import type { Snapshot } from './api/snapshot'
import { createAppRouter } from './router'

function stubFetch(response: () => Response) {
  vi.stubGlobal(
    'fetch',
    vi.fn<typeof fetch>(() => Promise.resolve(response())),
  )
}

function serve(snapshot: Snapshot) {
  stubFetch(
    () =>
      new Response(JSON.stringify(snapshot), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
  )
}

async function mountAt(path: string): Promise<VueWrapper> {
  const router = createAppRouter(createMemoryHistory())
  await router.push(path)
  await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } })
  await flushPromises()
  return wrapper
}

function card(wrapper: VueWrapper, sourceId: string) {
  const found = wrapper.find(`[data-test="source-card"][data-source="${sourceId}"]`)
  expect(found.exists(), `card for ${sourceId}`).toBe(true)
  return found
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('Overview (acceptance 3)', () => {
  it('shows the project name and root in the top bar', async () => {
    serve(allPresent)
    const wrapper = await mountAt('/')
    const top = wrapper.get('[data-test="top-bar"]')
    expect(top.text()).toContain('example')
    expect(top.text()).toContain('/work/example')
  })

  it('five Present sources render five cards, each naming its tool and tool_version', async () => {
    serve(allPresent)
    const wrapper = await mountAt('/')
    const cards = wrapper.findAll('[data-test="source-card"]')
    expect(cards).toHaveLength(5)
    for (const source of allPresent.sources) {
      const text = card(wrapper, source.source_id).text()
      expect(text).toContain(source.source_id)
      expect(text).toContain('present')
      if (source.tool !== null) {
        expect(text).toContain(source.tool)
        expect(text).toContain(source.tool_version ?? 'version unknown')
      } else {
        expect(text).toContain('no external tool')
      }
    }
  })

  it('Absent, ToolMissing and Failed are distinct and never empty or 0', async () => {
    serve(degraded)
    const wrapper = await mountAt('/')

    const plan = card(wrapper, 'plan')
    expect(plan.text()).toContain('tool missing: aep')
    expect(plan.attributes('data-availability')).toBe('ToolMissing')

    const spec = card(wrapper, 'spec')
    expect(spec.text()).toContain('failed')
    expect(spec.text()).toContain(FAILED_DIAGNOSTIC)
    expect(spec.attributes('data-availability')).toBe('Failed')

    const quality = card(wrapper, 'quality')
    expect(quality.text()).toContain('absent')
    expect(quality.attributes('data-availability')).toBe('Absent')

    for (const id of ['plan', 'spec', 'quality', 'docs']) {
      const status = card(wrapper, id).get('[data-test="source-status"]').text().trim()
      expect(status, `${id} status`).not.toBe('')
      expect(card(wrapper, id).text(), `${id} card`).not.toMatch(/(^|[^0-9.])0([^0-9.]|$)/)
    }
  })

  it('the vcs card shows summary.branch and the first 7 characters of summary.head', async () => {
    serve(degraded)
    const wrapper = await mountAt('/')
    const vcs = card(wrapper, 'vcs').text()
    expect(vcs).toContain('feature/x')
    expect(vcs).toContain(VCS_HEAD.slice(0, 7))
    expect(vcs).not.toContain(VCS_HEAD.slice(0, 8))
  })

  it('a 403 from /api/snapshot renders the token-rejected message', async () => {
    stubFetch(() => new Response('forbidden', { status: 403 }))
    const wrapper = await mountAt('/')
    expect(wrapper.text()).toContain('token rejected — reopen the URL repoview printed')
    expect(wrapper.findAll('[data-test="source-card"]')).toHaveLength(0)
  })

  it('a 403 on ./data/snapshot.json in static mode renders a load error naming 403, not the token message', async () => {
    const meta = document.createElement('meta')
    meta.name = 'repoview-mode'
    meta.content = 'static'
    document.head.append(meta)
    try {
      stubFetch(() => new Response('forbidden', { status: 403 }))
      const wrapper = await mountAt('/')
      expect(wrapper.text()).not.toContain('token rejected')
      expect(wrapper.get('[role="alert"]').text()).toBe('snapshot unavailable: HTTP 403')
    } finally {
      meta.remove()
    }
  })

  it('an Absent source the server names a tool for (quality: codegate) shows no tool line', async () => {
    serve(degraded)
    const wrapper = await mountAt('/')
    const quality = card(wrapper, 'quality')
    expect(quality.find('.tool').exists()).toBe(false)
    expect(quality.text()).not.toContain('codegate')
    expect(quality.text()).not.toContain('version unknown')
  })

  it('the app shell and the Overview page share one /api/snapshot fetch', async () => {
    serve(allPresent)
    await mountAt('/')
    expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1)
  })

  it('renders a loading notice until the snapshot arrives', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => new Promise<Response>(() => undefined)),
    )
    const wrapper = await mountAt('/')
    expect(wrapper.text()).toContain('loading snapshot…')
  })

  it('an empty project name or root is never an empty top-bar element', async () => {
    serve({ ...allPresent, project: { name: '', root: '' } })
    const wrapper = await mountAt('/')
    for (const selector of ['.project-name', '.project-root']) {
      expect(wrapper.get(selector).text().trim(), selector).not.toBe('')
    }
  })
})

describe('routing', () => {
  it('an unknown path renders the not-found page inside the layout', async () => {
    serve(allPresent)
    const wrapper = await mountAt('/no/such/page')
    expect(wrapper.get('[data-test="not-found"]').text()).toContain('not found')
    expect(wrapper.find('[data-test="top-bar"]').exists()).toBe(true)
    expect(wrapper.get('[data-test="nav"]').text()).toContain('Overview')
  })
})
