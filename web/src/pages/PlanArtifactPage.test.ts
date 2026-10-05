import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, type Router } from 'vue-router'
import artifactsFixture from '../__fixtures__/plan/artifacts.json'
import { failedWith } from '../__fixtures__/plan/failures'
import explainBlockedFixture from '../__fixtures__/plan/explain-blocked.json'
import explainFixture from '../__fixtures__/plan/explain.json'
import explainExecuted from '../__fixtures__/plan/explain-executed.json'
import historyExecuted from '../__fixtures__/plan/history-executed.json'
import historyFixture from '../__fixtures__/plan/history.json'
import showFixture from '../__fixtures__/plan/show.json'
import { apiGet, type ApiResult } from '../api/client'
import { createAppRouter } from '../router'
import PlanArtifactPage from './PlanArtifactPage.vue'

// story:plan-pages acceptance 4.

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  apiGet: vi.fn(),
}))

const ID = 'story:plan-pages'

// Captured: `aep plan artifact explain` of story:page-frame here, and of a blocked migration plan
// in aep's own store (`blocked_by` as `{ blocker, type }`, a `recorded_since` record).
const EXPLAIN = explainFixture
const BLOCKED = explainBlockedFixture

const FINDINGS = {
  findings: [
    {
      file: 'crates/repoview/src/api/plan.rs',
      line: 42,
      category: 'acceptance',
      severity: 'blocker',
      verdict: 'NEEDS-CHANGE',
      origin: 'introduced',
      message: 'the 502 drops the exit code',
    },
  ],
  outcomes: [
    {
      reviewed: ID,
      outcome: 'fixed',
      source: 'correction round 1',
      at: '2026-10-05T14:00:00Z',
    },
  ],
}

type Answers = Record<string, ApiResult<unknown>>

function ready(data: unknown): ApiResult<unknown> {
  return { state: 'ready', data }
}

function answers(id: string, show: unknown = showFixture): Answers {
  return {
    [`plan/artifacts/${id}`]: ready(show),
    [`plan/artifacts/${id}/history`]: ready([...historyFixture].reverse()),
    [`plan/artifacts/${id}/explain`]: ready({ ...EXPLAIN, artifact: id }),
    'plan/artifacts': ready(artifactsFixture),
  }
}

async function mountArtifact(
  id: string,
  given: Answers = answers(id),
): Promise<{ wrapper: VueWrapper; router: Router }> {
  vi.mocked(apiGet).mockImplementation((path: string) =>
    Promise.resolve(
      (given[path] ?? { state: 'error', status: 404, message: 'HTTP 404' }) as ApiResult<never>,
    ),
  )
  const router = createAppRouter(createMemoryHistory())
  await router.push(`/plan/artifact/${id}`)
  await router.isReady()
  const wrapper = mount(PlanArtifactPage, { global: { plugins: [router] } })
  await flushPromises()
  return { wrapper, router }
}

function texts(wrapper: VueWrapper, selector: string): string[] {
  return wrapper.findAll(selector).map((element) => element.text())
}

beforeEach(() => {
  vi.mocked(apiGet).mockReset()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Artifact page (acceptance 4)', () => {
  it('reads show, history, explain and the artifact list', async () => {
    await mountArtifact(ID)
    expect(
      vi
        .mocked(apiGet)
        .mock.calls.map((call) => call[0])
        .sort(),
    ).toEqual([
      'plan/artifacts',
      `plan/artifacts/${ID}`,
      `plan/artifacts/${ID}/explain`,
      `plan/artifacts/${ID}/history`,
    ])
  })

  it('shows title, kind, status, revision and summary', async () => {
    const { wrapper } = await mountArtifact(ID)
    expect(wrapper.get('[data-test="artifact-title"]').text()).toBe(showFixture.title)
    expect(wrapper.get('[data-test="artifact-kind"]').text()).toBe('story')
    expect(wrapper.get('[data-test="artifact-status"]').text()).toBe(showFixture.status)
    expect(wrapper.get('[data-test="artifact-revision"]').text()).toBe(
      `revision ${String(showFixture.revision)}`,
    )
    expect(wrapper.get('[data-test="artifact-summary"]').text()).toBe(showFixture.summary)
  })

  it('renders the body through MarkdownView', async () => {
    const { wrapper } = await mountArtifact(ID)
    const body = wrapper.get('[data-test="artifact-body"] [data-test="markdown"]')
    expect(body.find('h2').text()).toBe('Story')
    expect(body.find('table').exists()).toBe(true)
  })

  it('a hostile body does not reach the DOM', async () => {
    const show = {
      ...showFixture,
      body: '## Hi\n\n<img src=x onerror="alert(1)"><script>x</script>',
    }
    const { wrapper } = await mountArtifact(ID, { ...answers(ID, show) })
    const body = wrapper.get('[data-test="artifact-body"]')
    expect(body.element.querySelectorAll('script, img')).toHaveLength(0)
    expect(body.element.querySelectorAll('[onerror]')).toHaveLength(0)
  })

  it('outgoing relations link to their artifact pages', async () => {
    const { wrapper } = await mountArtifact(ID)
    const links = wrapper.findAll('[data-test="relations-out"] a')
    expect(links.map((a) => a.attributes('href'))).toEqual([
      '/plan/artifact/epic:plan',
      '/plan/artifact/vision:repoview',
      '/plan/artifact/story:page-frame',
    ])
    expect(texts(wrapper, '[data-test="relations-out"] [data-test="relation-name"]')).toEqual([
      'decomposes',
      'serves',
      'depends_on',
    ])
  })

  it('incoming relations link to the artifacts that point here', async () => {
    const { wrapper } = await mountArtifact(
      'epic:plan',
      answers('epic:plan', {
        ...showFixture,
        id: 'epic:plan',
        kind: 'epic',
      }),
    )
    const links = wrapper.findAll('[data-test="relations-in"] a')
    expect(links.map((a) => a.attributes('href'))).toEqual([
      '/plan/artifact/epic:static-export',
      '/plan/artifact/story:plan-pages',
    ])
  })

  it('following a relation loads that artifact', async () => {
    const both = {
      ...answers(ID),
      ...answers('epic:plan', { ...showFixture, id: 'epic:plan', title: 'The epic' }),
    }
    const { wrapper, router } = await mountArtifact(ID, both)
    await wrapper.get('[data-test="relations-out"] a').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/plan/artifact/epic:plan')
    expect(wrapper.get('[data-test="artifact-title"]').text()).toBe('The epic')
  })

  it('shows the scope with each line marked', async () => {
    const { wrapper } = await mountArtifact(ID)
    const rows = texts(wrapper, '[data-test="scope"] li')
    expect(rows).toHaveLength(showFixture.scope.length)
    expect(rows[0]).toContain('crates/repoview-sources/src/plan.rs')
    expect(rows[0]).toContain('cited')
  })

  it('history is oldest first', async () => {
    const { wrapper } = await mountArtifact(ID)
    const revisions = wrapper
      .findAll('[data-test="history"] [data-test="history-entry"]')
      .map((row) => row.attributes('data-revision'))
    expect(revisions).toEqual(['15', '16'])
    expect(wrapper.get('[data-test="history"]').text()).toContain('draft → proposed')
    expect(wrapper.get('[data-test="history"]').text()).toContain('human:timo')
  })

  it('shows the explain output: transitions reached and what is next', async () => {
    const { wrapper } = await mountArtifact(ID)
    const explain = wrapper.get('[data-test="explain"]')
    expect(explain.text()).toContain('draft → proposed')
    expect(explain.text()).toContain(EXPLAIN.reached[0]?.on_nothing_recorded)
    expect(texts(wrapper, '[data-test="explain-next"] > li')[0]).toContain('implemented')
    expect(texts(wrapper, '[data-test="explain-next"] > li')[0]).toContain('test_result')
  })

  it('explain blockers link to the blocker and name its type; records since are shown', async () => {
    const given = answers(ID)
    given[`plan/artifacts/${ID}/explain`] = ready(BLOCKED)
    const { wrapper } = await mountArtifact(ID, given)
    const blockers = wrapper.findAll('[data-test="explain-blocker"]')
    expect(blockers.map((b) => b.get('a').attributes('href'))).toEqual([
      '/plan/artifact/dependency-blocker:metaharness-source-receipt',
    ])
    expect(blockers[0]?.text()).toContain('dependency')
    const record = BLOCKED.recorded_since[0]
    expect(wrapper.get('[data-test="explain-recorded"]').text()).toContain(record?.source)
    expect(wrapper.get('[data-test="explain-recorded"]').text()).toContain(record?.kind)
  })

  it('explain says when planning documents could not be read', async () => {
    const given = answers(ID)
    given[`plan/artifacts/${ID}/explain`] = ready({ ...explainExecuted, unreadable: 2 })
    const { wrapper } = await mountArtifact(ID, given)
    expect(wrapper.get('[data-test="explain-unreadable"]').text()).toContain('2')
  })

  it('no unreadable notice when every document was read', async () => {
    const given = answers(ID)
    given[`plan/artifacts/${ID}/explain`] = ready(explainExecuted)
    const { wrapper } = await mountArtifact(ID, given)
    expect(wrapper.find('[data-test="explain-unreadable"]').exists()).toBe(false)
  })

  it('a step and a history move show their executor, correlation and evidence', async () => {
    const given = answers(ID)
    given[`plan/artifacts/${ID}/explain`] = ready(explainExecuted)
    given[`plan/artifacts/${ID}/history`] = ready(historyExecuted)
    const { wrapper } = await mountArtifact(ID, given)
    const explain = wrapper.get('[data-test="explain"]').text()
    expect(explain).toContain(
      'revision 4, executed by agent:codegate-wave001, correlation codegate-wave001',
    )
    expect(explain).toContain('(verification-report:codegate-wave1), observed 2026-10-02T12:55:17Z')
    const history = wrapper.get('[data-test="history"]').text()
    expect(history).toContain('executed by agent:codegate-wave001, correlation codegate-wave001')
    expect(history).toContain('test_result recorded from Single child story')
  })

  it('findings and outcomes show when present', async () => {
    const { wrapper } = await mountArtifact(ID, answers(ID, { ...showFixture, ...FINDINGS }))
    const finding = wrapper.get('[data-test="findings"]')
    expect(finding.text()).toContain('the 502 drops the exit code')
    expect(finding.text()).toContain('blocker')
    expect(finding.text()).toContain('crates/repoview/src/api/plan.rs:42')
    expect(wrapper.get('[data-test="outcomes"]').text()).toContain('correction round 1')
  })

  it('findings and outcomes are absent when there are none', async () => {
    const { wrapper } = await mountArtifact(ID)
    expect(wrapper.find('[data-test="findings"]').exists()).toBe(false)
    expect(wrapper.find('[data-test="outcomes"]').exists()).toBe(false)
  })

  it('a 502 on show shows the stderr text', async () => {
    const { wrapper } = await mountArtifact('story:missing', {
      'plan/artifacts/story:missing': failedWith(502, {
        tool: 'aep',
        exit: 1,
        stderr: 'error: the store holds no `story:missing`',
      }),
    })
    expect(wrapper.get('[data-test="tool-failure"]').text()).toContain(
      'error: the store holds no `story:missing`',
    )
  })

  it('a 502 on history or explain shows its stderr and keeps the rest of the page', async () => {
    const given = answers(ID)
    given[`plan/artifacts/${ID}/history`] = failedWith(502, {
      tool: 'aep',
      exit: 1,
      stderr: 'error: history unreadable',
    })
    const { wrapper } = await mountArtifact(ID, given)
    expect(wrapper.get('[data-test="history"]').text()).toContain('error: history unreadable')
    expect(wrapper.get('[data-test="artifact-title"]').text()).toBe(showFixture.title)
  })
})
