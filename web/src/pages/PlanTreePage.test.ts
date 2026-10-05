import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import artifactsFixture from '../__fixtures__/plan/artifacts.json'
import { failedWith } from '../__fixtures__/plan/failures'
import { apiGet, type ApiResult } from '../api/client'
import { createAppRouter } from '../router'
import PlanTreePage from './PlanTreePage.vue'

// story:plan-pages acceptance 3, against this repository's own store.

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  apiGet: vi.fn(),
}))

async function mountTree(
  result: ApiResult<unknown> = { state: 'ready', data: artifactsFixture },
): Promise<VueWrapper> {
  vi.mocked(apiGet).mockImplementation((path: string) =>
    Promise.resolve(
      (path === 'plan/artifacts'
        ? result
        : { state: 'error', status: 404, message: 'HTTP 404' }) as ApiResult<never>,
    ),
  )
  const router = createAppRouter(createMemoryHistory())
  await router.push('/plan/tree')
  await router.isReady()
  const wrapper = mount(PlanTreePage, { global: { plugins: [router] } })
  await flushPromises()
  return wrapper
}

/** The ids of the node elements directly below `node`'s child list. */
function childIds(node: VueWrapper | ReturnType<VueWrapper['get']>): string[] {
  const list = node.find(':scope > [data-test="tree-children"]')
  if (!list.exists()) return []
  return list
    .findAll(':scope > [data-test="tree-node"]')
    .map((child) => child.attributes('data-id') ?? '')
}

function rootNode(wrapper: VueWrapper, group: string, id: string) {
  return wrapper.get(`[data-test="${group}"] > [data-test="tree-node"][data-id="${id}"]`)
}

beforeEach(() => {
  vi.mocked(apiGet).mockReset()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Plan tree (acceptance 3)', () => {
  it('reads plan/artifacts', async () => {
    await mountTree()
    expect(vi.mocked(apiGet).mock.calls.map((call) => call[0])).toEqual(['plan/artifacts'])
  })

  it('the roots are the visions', async () => {
    const wrapper = await mountTree()
    const roots = wrapper.findAll('[data-test="tree-roots"] > [data-test="tree-node"]')
    expect(roots.map((root) => root.attributes('data-id'))).toEqual(['vision:repoview'])
  })

  it('vision → design → epic → story, each node linking to its artifact page', async () => {
    const wrapper = await mountTree()
    await wrapper.get('[data-test="expand-all"]').trigger('click')
    const vision = rootNode(wrapper, 'tree-roots', 'vision:repoview')
    expect(childIds(vision)).toContain('architecture-design:repoview')
    const design = vision.get(
      ':scope > [data-test="tree-children"] > [data-test="tree-node"][data-id="architecture-design:repoview"]',
    )
    expect(childIds(design)).toContain('epic:plan')
    const epic = design.get(
      ':scope > [data-test="tree-children"] > [data-test="tree-node"][data-id="epic:plan"]',
    )
    expect(childIds(epic)).toEqual(['story:plan-pages'])
    const story = epic.get('[data-test="tree-node"][data-id="story:plan-pages"]')
    expect(story.get('a').attributes('href')).toBe('/plan/artifact/story:plan-pages')
    expect(story.get('[data-test="tree-via"]').text()).toBe('decomposes')
  })

  it('an artifact reachable twice is shown under each parent', async () => {
    const wrapper = await mountTree()
    await wrapper.get('[data-test="expand-all"]').trigger('click')
    const shown = wrapper.findAll('[data-test="tree-node"][data-id="epic:plan"]')
    // Under the vision it serves and under the design it implements.
    expect(shown).toHaveLength(2)
  })

  it('artifacts with none of the edges and no children appear under Unattached', async () => {
    const wrapper = await mountTree()
    const group = wrapper.get('[data-test="tree-unattached"]')
    const ids = group
      .findAll(':scope > [data-test="tree-node"]')
      .map((node) => node.attributes('data-id'))
    expect(ids).toContain('review-result:shell-scope-round-1')
    expect(ids).toContain('executable-system-specification:read-model')
    expect(ids).not.toContain('story:plan-pages')
    expect(wrapper.get('[data-test="tree-unattached-heading"]').text()).toContain('Unattached')
  })

  it('a node collapses and expands', async () => {
    const wrapper = await mountTree()
    const vision = rootNode(wrapper, 'tree-roots', 'vision:repoview')
    expect(childIds(vision).length).toBeGreaterThan(0)
    await vision.get(':scope > .tree-row [data-test="tree-toggle"]').trigger('click')
    expect(childIds(vision)).toEqual([])
    await vision.get(':scope > .tree-row [data-test="tree-toggle"]').trigger('click')
    expect(childIds(vision).length).toBeGreaterThan(0)
  })

  it('collapse all leaves only the roots', async () => {
    const wrapper = await mountTree()
    await wrapper.get('[data-test="collapse-all"]').trigger('click')
    expect(childIds(rootNode(wrapper, 'tree-roots', 'vision:repoview'))).toEqual([])
  })

  it('a leaf has no toggle', async () => {
    const wrapper = await mountTree()
    const leaf = rootNode(wrapper, 'tree-unattached', 'review-result:shell-scope-round-1')
    expect(leaf.find('[data-test="tree-toggle"]').exists()).toBe(false)
  })

  it('a failed list shows the stderr aep printed', async () => {
    const wrapper = await mountTree(
      failedWith(502, { tool: 'aep', exit: 1, stderr: 'error: no planning store' }),
    )
    expect(wrapper.get('[data-test="tool-failure"]').text()).toContain('error: no planning store')
  })
})
