import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, type Router } from 'vue-router'
import App from './App.vue'
import { allPresent, degraded } from './__fixtures__/snapshots'
import type { Snapshot } from './api/snapshot'
import OverviewPage from './pages/OverviewPage.vue'
import PlanArtifactPage from './pages/PlanArtifactPage.vue'
import PlanBoardPage from './pages/PlanBoardPage.vue'
import PlanTreePage from './pages/PlanTreePage.vue'
import QualityPage from './pages/QualityPage.vue'
import RepositoryPage from './pages/RepositoryPage.vue'
import SpecRootPage from './pages/SpecRootPage.vue'
import SpecsPage from './pages/SpecsPage.vue'
import { createAppRouter } from './router'

// story:page-frame acceptance 3.

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

async function mountAt(path: string): Promise<{ wrapper: VueWrapper; router: Router }> {
  const router = createAppRouter(createMemoryHistory())
  await router.push(path)
  await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } })
  await flushPromises()
  return { wrapper, router }
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

const ROUTES = [
  { path: '/', component: OverviewPage, title: 'Overview' },
  { path: '/plan', component: PlanBoardPage, title: 'Plan · Board' },
  { path: '/plan/tree', component: PlanTreePage, title: 'Plan · Tree' },
  { path: '/plan/artifact/story:x', component: PlanArtifactPage, title: 'Plan · Artifact' },
  { path: '/specs', component: SpecsPage, title: 'Specs' },
  { path: '/specs/ess', component: SpecRootPage, title: 'Spec root' },
  { path: '/quality', component: QualityPage, title: 'Quality' },
  { path: '/repository', component: RepositoryPage, title: 'Repository' },
]

describe('routes', () => {
  for (const { path, component, title } of ROUTES) {
    it(`${path} renders its own page, titled "${title}", inside the layout`, async () => {
      serve(allPresent)
      const { wrapper, router } = await mountAt(path)
      expect(router.currentRoute.value.matched[0]?.components?.default).toBe(component)
      expect(wrapper.get('main h1').text()).toBe(title)
      expect(wrapper.find('[data-test="nav"]').exists()).toBe(true)
    })
  }

  it('/plan/artifact/:id carries the id', async () => {
    serve(allPresent)
    const { router } = await mountAt('/plan/artifact/story:page-frame')
    expect(router.currentRoute.value.params.id).toBe('story:page-frame')
  })

  it('/specs/:root(.*) carries a root with slashes', async () => {
    serve(allPresent)
    const { wrapper, router } = await mountAt('/specs/crates/a/ess')
    expect(router.currentRoute.value.params.root).toBe('crates/a/ess')
    expect(wrapper.get('main h1').text()).toBe('Spec root')
  })
})

function navEntries(wrapper: VueWrapper) {
  return wrapper.get('[data-test="nav"]').findAll('a')
}

function navEntry(wrapper: VueWrapper, title: string) {
  const entry = navEntries(wrapper).find((a) => a.attributes('data-nav') === title)
  if (entry === undefined) throw new Error(`no nav entry ${title}`)
  return entry
}

describe('left nav', () => {
  it('lists Overview, Plan, Specs, Quality, Repository, linking to their pages', async () => {
    serve(allPresent)
    const { wrapper } = await mountAt('/')
    const entries = navEntries(wrapper)
    expect(entries.map((a) => a.text())).toEqual([
      'Overview',
      'Plan',
      'Specs',
      'Quality',
      'Repository',
    ])
    expect(entries.map((a) => a.attributes('href'))).toEqual([
      '/',
      '/plan',
      '/specs',
      '/quality',
      '/repository',
    ])
    for (const a of entries) expect(a.attributes('data-absent')).toBe('false')
  })

  it('an Absent source dims its entry with "absent" and never hides it', async () => {
    serve(degraded)
    const { wrapper } = await mountAt('/')
    expect(navEntries(wrapper)).toHaveLength(5)

    const quality = navEntry(wrapper, 'Quality')
    expect(quality.attributes('data-absent')).toBe('true')
    expect(quality.classes()).toContain('nav-absent')
    expect(quality.text()).toBe('Quality absent')
    expect(quality.attributes('href')).toBe('/quality')

    // ToolMissing (plan) and Failed (spec) are not Absent.
    for (const title of ['Plan', 'Specs']) {
      expect(navEntry(wrapper, title).attributes('data-absent'), title).toBe('false')
      expect(navEntry(wrapper, title).text(), title).toBe(title)
    }
    // Repository reads vcs and docs; docs Absent with vcs Present is not an absent page.
    expect(navEntry(wrapper, 'Repository').attributes('data-absent')).toBe('false')
  })

  it('Repository is absent when both vcs and docs are Absent', async () => {
    serve({
      ...degraded,
      sources: degraded.sources.map((s) =>
        s.source_id === 'vcs' ? { ...s, availability: 'Absent' } : s,
      ),
    })
    const { wrapper } = await mountAt('/')
    const repository = navEntry(wrapper, 'Repository')
    expect(repository.attributes('data-absent')).toBe('true')
    expect(repository.text()).toBe('Repository absent')
  })

  it('no entry is dimmed while the snapshot loads or failed to load', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => Promise.resolve(new Response('no', { status: 500 }))),
    )
    const { wrapper } = await mountAt('/')
    expect(navEntries(wrapper)).toHaveLength(5)
    for (const a of navEntries(wrapper)) expect(a.attributes('data-absent')).toBe('false')
  })
})
