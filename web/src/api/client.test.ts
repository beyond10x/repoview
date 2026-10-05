import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TOKEN_HEADER, TOKEN_STORAGE_KEY, apiGet, forgetToken } from './client'

// story:page-frame acceptance 2.

const TOKEN = 'c'.repeat(64)

function stubFetch(response: () => Response) {
  const fetchMock = vi.fn<typeof fetch>(() => Promise.resolve(response()))
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function onlyCall(fetchMock: ReturnType<typeof stubFetch>): { url: string; token: string | null } {
  expect(fetchMock).toHaveBeenCalledTimes(1)
  const call = fetchMock.mock.calls[0]
  if (call === undefined) throw new Error('fetch was not called')
  const [input, init] = call
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  return { url, token: new Headers(init?.headers).get(TOKEN_HEADER) }
}

function setMode(content: string | null) {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  if (content === null) return
  const meta = document.createElement('meta')
  meta.name = 'repoview-mode'
  meta.content = content
  document.head.append(meta)
}

beforeEach(() => {
  forgetToken()
  sessionStorage.clear()
  sessionStorage.setItem(TOKEN_STORAGE_KEY, TOKEN)
})

afterEach(() => {
  setMode(null)
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('apiGet in server mode', () => {
  beforeEach(() => {
    setMode('server')
  })

  it('fetches /api/<path> with X-Repoview-Token and answers ready with the body', async () => {
    const fetchMock = stubFetch(() => json({ columns: [] }))
    expect(await apiGet<{ columns: unknown[] }>('plan/board')).toEqual({
      state: 'ready',
      data: { columns: [] },
    })
    expect(onlyCall(fetchMock)).toEqual({ url: '/api/plan/board', token: TOKEN })
  })

  it('URL-encodes each path segment and keeps /', async () => {
    const fetchMock = stubFetch(() => json({}))
    await apiGet('plan/artifacts/story:a b?#%')
    expect(onlyCall(fetchMock).url).toBe('/api/plan/artifacts/story%3Aa%20b%3F%23%25')
  })

  it('a 403 is token-rejected', async () => {
    stubFetch(() => new Response('forbidden', { status: 403 }))
    expect(await apiGet('plan/board')).toEqual({ state: 'token-rejected' })
  })

  it('any other non-2xx is error with the status', async () => {
    for (const status of [400, 404, 500, 503]) {
      stubFetch(() => new Response('no', { status }))
      const result = await apiGet('plan/board')
      expect(result).toEqual({ state: 'error', status, message: `HTTP ${String(status)}` })
    }
  })

  it('a non-2xx with a JSON body carries that body', async () => {
    const body = { tool: 'aep', exit: 1, stderr: 'error: no such artifact\n' }
    stubFetch(() => json(body, 502))
    expect(await apiGet('plan/artifacts/story:x')).toEqual({
      state: 'error',
      status: 502,
      message: 'HTTP 502',
      body,
    })
  })

  it('a non-2xx without a JSON body carries no body', async () => {
    stubFetch(() => new Response('no', { status: 502 }))
    const result = await apiGet('plan/board')
    expect(result).toEqual({ state: 'error', status: 502, message: 'HTTP 502' })
    expect('body' in result).toBe(false)
  })

  it('a network failure is error without a status, carrying the reason', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => Promise.reject(new TypeError('network down'))),
    )
    const result = await apiGet('plan/board')
    expect(result.state).toBe('error')
    expect(result.state === 'error' && result.status).toBeNull()
    expect(result.state === 'error' && result.message).toContain('network down')
  })

  it('a 2xx body that is not JSON is error, never a throw', async () => {
    stubFetch(() => new Response('<html>', { status: 200 }))
    const result = await apiGet('plan/board')
    expect(result.state).toBe('error')
  })

  it('with no stored token sends no token header', async () => {
    forgetToken()
    const fetchMock = stubFetch(() => json({}))
    await apiGet('plan/board')
    expect(onlyCall(fetchMock)).toEqual({ url: '/api/plan/board', token: null })
  })
})

describe('apiGet in static mode', () => {
  beforeEach(() => {
    setMode('static')
  })

  it('fetches ./data/<path>.json with no token and answers ready with the body', async () => {
    const fetchMock = stubFetch(() => json({ columns: [] }))
    expect(await apiGet('plan/board')).toEqual({ state: 'ready', data: { columns: [] } })
    const call = onlyCall(fetchMock)
    expect(call).toEqual({ url: './data/plan/board.json', token: null })
    expect(JSON.stringify(fetchMock.mock.calls)).not.toContain(TOKEN)
  })

  it('URL-encodes each path segment and keeps /', async () => {
    const fetchMock = stubFetch(() => json({}))
    await apiGet('spec/roots/my root:x/ir')
    expect(onlyCall(fetchMock).url).toBe('./data/spec/roots/my%20root%3Ax/ir.json')
  })

  it('a 403 is error with status 403, not token-rejected', async () => {
    stubFetch(() => new Response('forbidden', { status: 403 }))
    expect(await apiGet('plan/board')).toEqual({ state: 'error', status: 403, message: 'HTTP 403' })
  })

  it('any other non-2xx is error with the status', async () => {
    for (const status of [404, 500]) {
      stubFetch(() => new Response('no', { status }))
      expect(await apiGet('plan/board')).toEqual({
        state: 'error',
        status,
        message: `HTTP ${String(status)}`,
      })
    }
  })

  it('a network failure is error without a status, carrying the reason', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => Promise.reject(new TypeError('offline'))),
    )
    const result = await apiGet('plan/board')
    expect(result.state).toBe('error')
    expect(result.state === 'error' && result.status).toBeNull()
    expect(result.state === 'error' && result.message).toContain('offline')
  })
})

describe('apiGet refuses a path segment that is empty, "." or ".." after decoding', () => {
  for (const mode of ['server', 'static'] as const) {
    it(`${mode} mode: answers error and makes no request`, async () => {
      setMode(mode)
      const fetchMock = stubFetch(() => json({}))
      for (const path of [
        'spec/roots/../ir',
        'spec/roots/./ir',
        'spec/roots//ir',
        'spec/roots/ir/',
        '/spec',
        '..',
        '',
        'spec/roots/%2e%2e/ir',
        'spec/roots/%2E/ir',
        'spec/roots/.%2e/ir',
      ]) {
        const result = await apiGet(path)
        expect(result.state, path).toBe('error')
        expect(result.state === 'error' && result.status, path).toBeNull()
        expect(result.state === 'error' && result.message, path).toContain('refused')
      }
      expect(fetchMock).not.toHaveBeenCalled()
    })
  }

  it('still fetches segments that only contain dots among other characters', async () => {
    setMode('server')
    const fetchMock = stubFetch(() => json({}))
    await apiGet('spec/roots/.ess/a..b/...')
    expect(onlyCall(fetchMock).url).toBe('/api/spec/roots/.ess/a..b/...')
  })
})
