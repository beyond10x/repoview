import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  TOOL_PATH,
  everyStatus,
  markdownAssessment,
  settled,
  strayAssessments,
} from '../__fixtures__/quality/reports'
import { TOKEN_HEADER, TOKEN_STORAGE_KEY } from '../api/client'
import type { QualityReport } from '../api/quality'
import QualityPage from './QualityPage.vue'

// story:quality-page acceptance 3.

function respond(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

/** Each fetch answers with the next of `bodies`; the last one repeats. */
function serve(...bodies: Array<[number, unknown]>) {
  let call = 0
  const fetchMock = vi.fn<typeof fetch>(() => {
    const [status, body] = bodies[Math.min(call, bodies.length - 1)] ?? [500, null]
    call += 1
    return Promise.resolve(respond(status, body))
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

async function mountPage(): Promise<VueWrapper> {
  const wrapper = mount(QualityPage)
  await flushPromises()
  return wrapper
}

function card(wrapper: VueWrapper, language: string) {
  return wrapper.get(`[data-test="language-card"][data-language="${language}"]`)
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('the Quality page reads /api/quality', () => {
  it('fetches /api/quality with the run token', async () => {
    sessionStorage.setItem(TOKEN_STORAGE_KEY, 'tok')
    const fetchMock = serve([200, settled])
    await mountPage()
    const [url, init] = fetchMock.mock.calls[0] ?? []
    expect(url).toBe('/api/quality')
    expect((init?.headers as Record<string, string>)[TOKEN_HEADER]).toBe('tok')
  })

  it('keeps its title and names the producing binary', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    expect(wrapper.get('h1').text()).toBe('Quality')
    const producer = wrapper.get('[data-test="producer"]').text()
    expect(producer).toContain('codegate')
    expect(producer).toContain(TOOL_PATH)
  })

  it('renders one card per language, in server order', async () => {
    serve([200, everyStatus])
    const wrapper = await mountPage()
    const cards = wrapper.findAll('[data-test="language-card"]')
    expect(cards.map((c) => c.attributes('data-language'))).toEqual([
      'go',
      'rust',
      'java',
      'markdown',
    ])
    expect(cards.map((c) => c.attributes('data-status'))).toEqual([
      'running',
      'not-assessed',
      'failed',
      'assessed',
    ])
  })

  it('a project with no detected language says so', async () => {
    serve([200, { ...settled, languages: [] } satisfies QualityReport])
    const wrapper = await mountPage()
    expect(wrapper.findAll('[data-test="language-card"]')).toHaveLength(0)
    expect(wrapper.get('[data-test="no-languages"]').text()).toContain('no language')
  })
})

describe('an assessed language (fixture captured from codegate on this repository)', () => {
  it('shows the rating', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    expect(card(wrapper, 'markdown').get('[data-test="rating"]').text()).toBe('B-')
  })

  it('shows every score as a bar from 0 to score_max, overall first', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    const bars = card(wrapper, 'markdown').findAll('[data-test="score"]')
    const scores = markdownAssessment.scores as Record<string, number>
    expect(bars.map((bar) => bar.attributes('data-score'))).toEqual(Object.keys(scores))
    expect(bars[0]?.attributes('data-score')).toBe('overall')
    for (const bar of bars) {
      const name = bar.attributes('data-score') ?? ''
      const value = scores[name] ?? NaN
      const meter = bar.get('[role="meter"]')
      expect(meter.attributes('aria-valuemin')).toBe('0')
      expect(meter.attributes('aria-valuemax')).toBe('100')
      expect(meter.attributes('aria-valuenow')).toBe(String(value))
      expect(bar.get('.fill').attributes('style')).toContain(`width: ${String(value)}%`)
      expect(bar.get('.value').text()).toBe(String(value))
    }
  })

  it('scales the bars to score_max, not to 100', async () => {
    const assessment = { ...markdownAssessment, score_max: 10, scores: { overall: 5 } }
    const report: QualityReport = {
      ...settled,
      languages: [{ language: 'go', status: 'assessed', reason: null, assessment }],
    }
    serve([200, report])
    const wrapper = await mountPage()
    const bar = card(wrapper, 'go').get('[data-test="score"]')
    expect(bar.get('[role="meter"]').attributes('aria-valuemax')).toBe('10')
    expect(bar.get('.fill').attributes('style')).toContain('width: 50%')
  })

  it('shows the summary', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    const summary = card(wrapper, 'markdown').get('[data-test="summary"]').text()
    expect(summary).toContain('findings')
    expect(summary).toContain('30')
    expect(summary).toContain('packages')
    expect(summary).toContain('34')
  })

  it('shows the finding counts, largest first', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    const rows = card(wrapper, 'markdown').findAll('[data-test="finding-count"]')
    expect(rows.map((row) => row.attributes('data-kind'))).toEqual([
      'markdown_missing_h1',
      'markdown_large_section',
    ])
    expect(rows[0]?.text()).toContain('29')
    expect(rows[1]?.text()).toContain('1')
  })

  it('shows each top finding with its title and location', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    const findings = card(wrapper, 'markdown').findAll('[data-test="top-finding"]')
    expect(findings).toHaveLength((markdownAssessment.top_findings as unknown[]).length)
    const first = findings[0]
    expect(first?.get('[data-test="finding-title"]').text()).toBe('Document has no H1 title.')
    // codegate's ranges are 0-based; the page shows 1-based lines.
    expect(first?.get('[data-test="finding-location"]').text()).toBe(
      '.engineering/planning/review-result/shell-acceptance-round-1.md:1',
    )
    expect(first?.text()).toContain('warning')
  })

  it('a finding with a title of its own shows that title', async () => {
    const assessment = {
      ...markdownAssessment,
      top_findings: [{ kind: 'k', title: 'Own title', reason: 'why', location: { uri: 'a.go' } }],
    }
    serve([
      200,
      { ...settled, languages: [{ language: 'go', status: 'assessed', reason: null, assessment }] },
    ])
    const wrapper = await mountPage()
    const finding = card(wrapper, 'go').get('[data-test="top-finding"]')
    expect(finding.get('[data-test="finding-title"]').text()).toBe('Own title')
    expect(finding.get('[data-test="finding-location"]').text()).toBe('a.go')
  })
})

describe('a language that was not assessed', () => {
  it('not-assessed shows the reason', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    expect(card(wrapper, 'rust').get('[data-test="reason"]').text()).toContain(
      'codegate does not support rust',
    )
  })

  it('failed shows the reason and the stderr', async () => {
    serve([200, settled])
    const wrapper = await mountPage()
    const java = card(wrapper, 'java')
    expect(java.get('[data-test="reason"]').text()).toContain('codegate assess failed')
    expect(java.get('pre[data-test="stderr"]').text()).toBe(
      'codegate: java backend crashed\npanic: index out of range',
    )
  })

  it('running shows a spinner', async () => {
    serve([200, everyStatus])
    const wrapper = await mountPage()
    expect(card(wrapper, 'go').find('[data-test="spinner"]').exists()).toBe(true)
    expect(card(wrapper, 'markdown').find('[data-test="spinner"]').exists()).toBe(false)
  })

  it('never shows a score, rating or finding, even when the server attaches an assessment', async () => {
    serve([200, strayAssessments])
    const wrapper = await mountPage()
    for (const language of ['go', 'rust', 'java']) {
      const c = card(wrapper, language)
      expect(c.find('[data-test="rating"]').exists(), language).toBe(false)
      expect(c.find('[data-test="score"]').exists(), language).toBe(false)
      expect(c.find('[data-test="finding-count"]').exists(), language).toBe(false)
      expect(c.find('[data-test="top-finding"]').exists(), language).toBe(false)
      expect(c.text(), language).not.toContain('B-')
    }
    expect(card(wrapper, 'markdown').findAll('[data-test="score"]').length).toBeGreaterThan(0)
  })
})

describe('polling', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
  })

  it('polls every 2 s while a language is running, then stops', async () => {
    const fetchMock = serve([200, everyStatus], [200, everyStatus], [200, settled])
    const wrapper = await mountPage()
    expect(fetchMock).toHaveBeenCalledTimes(1)
    await vi.advanceTimersByTimeAsync(1999)
    expect(fetchMock).toHaveBeenCalledTimes(1)
    await vi.advanceTimersByTimeAsync(1)
    await flushPromises()
    expect(fetchMock).toHaveBeenCalledTimes(2)
    await vi.advanceTimersByTimeAsync(2000)
    await flushPromises()
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(card(wrapper, 'go').attributes('data-status')).toBe('assessed')
    expect(card(wrapper, 'go').find('[data-test="spinner"]').exists()).toBe(false)
    await vi.advanceTimersByTimeAsync(10000)
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })

  it('does not poll when nothing is running', async () => {
    const fetchMock = serve([200, settled])
    await mountPage()
    await vi.advanceTimersByTimeAsync(10000)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('stops polling when the page is left', async () => {
    const fetchMock = serve([200, everyStatus])
    const wrapper = await mountPage()
    wrapper.unmount()
    await vi.advanceTimersByTimeAsync(10000)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('a failed poll keeps the last report and keeps polling', async () => {
    const fetchMock = serve([200, everyStatus], [500, 'boom'], [200, settled])
    const wrapper = await mountPage()
    await vi.advanceTimersByTimeAsync(2000)
    await flushPromises()
    expect(card(wrapper, 'go').attributes('data-status')).toBe('running')
    expect(wrapper.get('[data-test="poll-error"]').text()).toContain('500')
    await vi.advanceTimersByTimeAsync(2000)
    await flushPromises()
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(card(wrapper, 'go').attributes('data-status')).toBe('assessed')
    expect(wrapper.find('[data-test="poll-error"]').exists()).toBe(false)
  })
})

describe('no assessment at all', () => {
  it('503 says codegate is not on PATH, with no card', async () => {
    serve([503, { tool: 'codegate', exit: null, stderr: 'codegate not found on PATH' }])
    const wrapper = await mountPage()
    expect(wrapper.get('[data-test="unavailable"]').text()).toContain('codegate not found on PATH')
    expect(wrapper.findAll('[data-test="language-card"]')).toHaveLength(0)
  })

  it('403 says the token was rejected', async () => {
    serve([403, 'forbidden'])
    const wrapper = await mountPage()
    expect(wrapper.get('[role="alert"]').text()).toContain('token rejected')
  })

  it('any other error names the status', async () => {
    serve([500, 'boom'])
    const wrapper = await mountPage()
    expect(wrapper.get('[role="alert"]').text()).toContain('500')
  })

  it('a document that is not a quality report is an error, not an empty page', async () => {
    serve([200, { sources: [] }])
    const wrapper = await mountPage()
    expect(wrapper.get('[role="alert"]').text()).toContain('/api/quality')
    expect(wrapper.findAll('[data-test="language-card"]')).toHaveLength(0)
  })
})
