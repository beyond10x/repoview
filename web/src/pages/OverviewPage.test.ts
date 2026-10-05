import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, type Router } from 'vue-router'
import App from '../App.vue'
import { allPresent, degraded } from '../__fixtures__/snapshots'
import type { Snapshot } from '../api/snapshot'
import { createAppRouter } from '../router'

// story:page-frame acceptance 6.

function serve(snapshot: Snapshot) {
  vi.stubGlobal(
    'fetch',
    vi.fn<typeof fetch>(() =>
      Promise.resolve(
        new Response(JSON.stringify(snapshot), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    ),
  )
}

async function mountOverview(snapshot: Snapshot): Promise<{ wrapper: VueWrapper; router: Router }> {
  serve(snapshot)
  const router = createAppRouter(createMemoryHistory())
  await router.push('/')
  await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } })
  await flushPromises()
  return { wrapper, router }
}

function cardLink(wrapper: VueWrapper, sourceId: string) {
  return wrapper.get(`[data-test="source-card"][data-source="${sourceId}"]`).find('a')
}

const PAGES: Record<string, string> = {
  vcs: '/repository',
  docs: '/repository',
  plan: '/plan',
  spec: '/specs',
  quality: '/quality',
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('Overview cards link to their page', () => {
  for (const [fixture, snapshot] of [
    ['all present', allPresent],
    ['degraded', degraded],
  ] as const) {
    it(`every card links to its page (${fixture})`, async () => {
      const { wrapper } = await mountOverview(snapshot)
      for (const [sourceId, page] of Object.entries(PAGES)) {
        const link = cardLink(wrapper, sourceId)
        expect(link.exists(), sourceId).toBe(true)
        expect(link.attributes('href'), sourceId).toBe(page)
      }
    })
  }

  it('following a card link opens that page', async () => {
    const { wrapper, router } = await mountOverview(allPresent)
    await cardLink(wrapper, 'spec').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/specs')
    expect(wrapper.get('main h1').text()).toBe('Specs')
  })

  it('a source with no page renders its card without a link', async () => {
    const extra = { ...allPresent.sources[0], source_id: 'other' } as Snapshot['sources'][number]
    const { wrapper } = await mountOverview({
      ...allPresent,
      sources: [...allPresent.sources, extra],
    })
    expect(wrapper.find('[data-test="source-card"][data-source="other"]').exists()).toBe(true)
    expect(cardLink(wrapper, 'other').exists()).toBe(false)
  })
})
