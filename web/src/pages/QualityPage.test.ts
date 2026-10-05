import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  GO_PATH,
  REASON,
  TOOL_PATH,
  codegate030,
  goSkipped,
  strayScores,
} from '../__fixtures__/quality/reports'
import { TOKEN_HEADER, TOKEN_STORAGE_KEY } from '../api/client'
import QualityPage from './QualityPage.vue'

// story:quality-codegate acceptance 3.

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

/** Nothing on the page reads as a score, rating or finding. */
function expectNoScores(wrapper: VueWrapper): void {
  for (const test of ['rating', 'score', 'finding-count', 'top-finding', 'language-card']) {
    expect(wrapper.find(`[data-test="${test}"]`).exists(), test).toBe(false)
  }
  expect(wrapper.find('[role="meter"]').exists()).toBe(false)
  expect(wrapper.text()).not.toContain('B-')
  expect(wrapper.text()).not.toContain('67')
  expect(wrapper.text()).not.toContain('Document has no H1 title.')
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
    const fetchMock = serve([200, codegate030])
    await mountPage()
    const [url, init] = fetchMock.mock.calls[0] ?? []
    expect(url).toBe('/api/quality')
    expect((init?.headers as Record<string, string>)[TOKEN_HEADER]).toBe('tok')
  })

  it('asks once and does not poll: no assessment is running', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval'] })
    const fetchMock = serve([200, codegate030])
    await mountPage()
    await vi.advanceTimersByTimeAsync(10000)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })
})

describe('with codegate 0.3.0 on PATH', () => {
  it('keeps its title and names the producing binary and its version', async () => {
    serve([200, codegate030])
    const wrapper = await mountPage()
    expect(wrapper.get('h1').text()).toBe('Quality')
    const producer = wrapper.get('[data-test="producer"]')
    expect(producer.text()).toContain('codegate')
    expect(producer.get('[data-test="tool-version"]').text()).toBe('0.3.0')
    expect(producer.get('[data-test="tool-path"]').text()).toBe(TOOL_PATH)
  })

  it('lists the commands the server read from codegate --help, in order', async () => {
    serve([200, { ...codegate030, commands: ['evaluate', 'survey'] }])
    const wrapper = await mountPage()
    const commands = wrapper.findAll('[data-test="command"]').map((c) => c.text())
    expect(commands).toEqual(['evaluate', 'survey'])
  })

  it('a codegate offering no commands says so', async () => {
    serve([200, { ...codegate030, commands: [] }])
    const wrapper = await mountPage()
    expect(wrapper.findAll('[data-test="command"]')).toHaveLength(0)
    expect(wrapper.get('[data-test="no-commands"]').text()).toContain('no commands')
  })

  it('shows the reason there is no assessment', async () => {
    serve([200, codegate030])
    const wrapper = await mountPage()
    expect(wrapper.get('[data-test="reason"]').text()).toBe(REASON)
  })

  it('links to the beyond10x codegate repository', async () => {
    serve([200, codegate030])
    const wrapper = await mountPage()
    const link = wrapper.get('a[data-test="codegate-link"]')
    expect(link.attributes('href')).toBe('https://github.com/beyond10x/codegate')
  })

  it('names a skipped codegate, and shows no skipped list when none was', async () => {
    serve([200, goSkipped])
    const wrapper = await mountPage()
    const skipped = wrapper.findAll('[data-test="skipped-path"]').map((s) => s.text())
    expect(skipped).toEqual([GO_PATH])
    expect(wrapper.get('[data-test="tool-path"]').text()).toBe(TOOL_PATH)

    serve([200, codegate030])
    const plain = await mountPage()
    expect(plain.find('[data-test="skipped"]').exists()).toBe(false)
  })

  it('shows no score, rating or finding without an assessment', async () => {
    serve([200, codegate030])
    expectNoScores(await mountPage())
  })

  it('shows no score, rating or finding even when the document carries them', async () => {
    serve([200, strayScores])
    const wrapper = await mountPage()
    expect(wrapper.get('[data-test="reason"]').text()).toBe(REASON)
    expectNoScores(wrapper)
  })
})

describe('no codegate answer', () => {
  it('503 shows the server stderr text, with no producer and no score', async () => {
    serve([503, { tool: 'codegate', exit: null, stderr: 'beyond10x codegate not found on PATH' }])
    const wrapper = await mountPage()
    expect(wrapper.get('[data-test="unavailable"]').text()).toContain(
      'beyond10x codegate not found on PATH',
    )
    expect(wrapper.find('[data-test="producer"]').exists()).toBe(false)
    expectNoScores(wrapper)
  })

  it('503 shows whatever stderr the server sent, not a text of its own', async () => {
    serve([503, { tool: 'codegate', exit: null, stderr: 'a different diagnostic' }])
    const wrapper = await mountPage()
    expect(wrapper.get('[data-test="unavailable"]').text()).toContain('a different diagnostic')
  })

  it('503 links to the beyond10x codegate repository', async () => {
    serve([503, { tool: 'codegate', exit: null, stderr: 'beyond10x codegate not found on PATH' }])
    const wrapper = await mountPage()
    expect(wrapper.get('a[data-test="codegate-link"]').attributes('href')).toBe(
      'https://github.com/beyond10x/codegate',
    )
  })

  it('a 502 from a failing codegate --help shows its stderr', async () => {
    serve([502, { tool: 'codegate', exit: null, stderr: 'help broke' }])
    const wrapper = await mountPage()
    const alert = wrapper.get('[role="alert"]').text()
    expect(alert).toContain('502')
    expect(alert).toContain('help broke')
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
    expect(wrapper.find('[data-test="producer"]').exists()).toBe(false)
  })
})
