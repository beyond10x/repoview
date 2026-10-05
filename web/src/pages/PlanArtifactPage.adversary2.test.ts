import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import artifactsFixture from '../__fixtures__/plan/artifacts.json'
import showFixture from '../__fixtures__/plan/show.json'
import { apiGet, type ApiResult } from '../api/client'
import { createAppRouter } from '../router'
import PlanArtifactPage from './PlanArtifactPage.vue'

// Adversary, story:plan-pages, pass 2. Acceptance 4 asks for the explain output. Both steps below
// are as `aep plan artifact explain --format json` (aep 0.68.0) printed them from real stores:
// the first from this repository's own store (story:server-skeleton), the second from codegate's
// (epic:offline-dependency-evaluation). aep's own text rendering of the second prints
// "(revision 4, executed by agent:codegate-wave001, correlation codegate-wave001)".

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  apiGet: vi.fn(),
}))

const ID = 'story:plan-pages'

const EXPLAIN = {
  artifact: ID,
  store: '/work/example/.engineering/planning',
  status: 'implemented',
  revision: 10,
  blocked_by: [],
  reached: [
    {
      from: 'draft',
      to: 'proposed',
      at: '2026-10-05T12:37:43Z',
      revision: 10,
      rested_on: [
        {
          kind: 'review_outcome',
          source: 'review-result:shell-acceptance-round-1',
          at: '2026-10-05T12:36:17Z',
          revision: 7,
        },
        {
          kind: 'review_outcome',
          source: 'review-result:shell-design-round-2',
          at: '2026-10-05T12:37:39Z',
          revision: 9,
        },
      ],
    },
    {
      from: 'active',
      to: 'implemented',
      at: '2026-10-02T12:55:18Z',
      revision: 4,
      rested_on: [],
      on_nothing_recorded: 'no record: nothing was recorded about how this was decided',
      executor: 'agent:codegate-wave001',
      correlation: 'codegate-wave001',
    },
  ],
  recorded_since: [],
  next: [],
  unreadable: 0,
}

afterEach(() => {
  vi.restoreAllMocks()
})

async function mountWithExplain() {
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
  return wrapper.get('[data-test="explain"]').text()
}

describe('explain steps as aep prints them', () => {
  it('shows each record a move rested on, as text', async () => {
    const text = await mountWithExplain()
    expect(text).toContain('review_outcome')
    expect(text).toContain('review-result:shell-acceptance-round-1')
    expect(text).toContain('review-result:shell-design-round-2')
    expect(text).not.toContain('[object Object]')
  })

  it('names what executed a move when that was not its actor', async () => {
    const text = await mountWithExplain()
    expect(text).toContain('agent:codegate-wave001')
  })
})
