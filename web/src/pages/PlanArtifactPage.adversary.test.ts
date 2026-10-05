import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import artifactsFixture from '../__fixtures__/plan/artifacts.json'
import showFixture from '../__fixtures__/plan/show.json'
import { apiGet, type ApiResult } from '../api/client'
import { createAppRouter } from '../router'
import PlanArtifactPage from './PlanArtifactPage.vue'

// Adversary, story:plan-pages. `aep plan artifact explain --format json` (aep 0.68.0) prints
// `blocked_by` as objects `{ blocker, type, withholds? }` (aep-cli `Blocking`), not ids. The unit's
// own EXPLAIN fixture invented `blocked_by: ['story:page-frame']`.

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  apiGet: vi.fn(),
}))

const ID = 'story:plan-pages'
const BLOCKER = 'dependency-blocker:metaharness-source-receipt'

const EXPLAIN = {
  artifact: ID,
  store: '/work/example/.engineering/planning',
  status: 'active',
  revision: 4,
  blocked_by: [{ blocker: BLOCKER, type: 'dependency' }],
  reached: [],
  recorded_since: [],
  next: [],
  unreadable: 0,
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('explain blockers as aep prints them', () => {
  it('link to the blocker artifact page and name it', async () => {
    const given: Record<string, ApiResult<unknown>> = {
      [`plan/artifacts/${ID}`]: { state: 'ready', data: showFixture },
      [`plan/artifacts/${ID}/history`]: { state: 'ready', data: [] },
      [`plan/artifacts/${ID}/explain`]: { state: 'ready', data: EXPLAIN },
      'plan/artifacts': { state: 'ready', data: artifactsFixture },
    }
    vi.mocked(apiGet).mockImplementation((path: string) =>
      Promise.resolve(
        (given[path] ?? { state: 'error', status: 404, message: 'HTTP 404' }) as ApiResult<never>,
      ),
    )
    const router = createAppRouter(createMemoryHistory())
    await router.push(`/plan/artifact/${ID}`)
    await router.isReady()
    const wrapper = mount(PlanArtifactPage, { global: { plugins: [router] } })
    await flushPromises()

    const hrefs = wrapper
      .get('[data-test="explain"]')
      .findAll('a')
      .map((a) => a.attributes('href'))
    expect(hrefs).toContain(`/plan/artifact/${BLOCKER}`)
    expect(hrefs.some((href) => href?.includes('object'))).toBe(false)
  })
})
