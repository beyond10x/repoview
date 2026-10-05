import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { allPresent } from '../__fixtures__/snapshots'
import {
  TOKEN_HEADER,
  TOKEN_STORAGE_KEY,
  captureToken,
  forgetToken,
  loadSnapshot,
  readMode,
} from './snapshot'

const TOKEN = 'a'.repeat(64)

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function stubFetch(response: () => Response) {
  const fetchMock = vi.fn<typeof fetch>(() => Promise.resolve(response()))
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

/** The one fetch the client made: its URL as given, and its init. */
function onlyFetch(fetchMock: ReturnType<typeof stubFetch>): { url: string; init?: RequestInit } {
  expect(fetchMock).toHaveBeenCalledTimes(1)
  const call = fetchMock.mock.calls[0]
  if (call === undefined) throw new Error('fetch was not called')
  const [input, init] = call
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  return { url, init }
}

function setMode(content: string | null) {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  if (content !== null) {
    const meta = document.createElement('meta')
    meta.name = 'repoview-mode'
    meta.content = content
    document.head.append(meta)
  }
}

function headerOf(init: RequestInit | undefined, name: string): string | null {
  return new Headers(init?.headers).get(name)
}

beforeEach(() => {
  forgetToken()
  sessionStorage.clear()
  setMode(null)
  window.history.replaceState(null, '', '/')
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('token transport (acceptance 4)', () => {
  it('moves ?token into sessionStorage, strips it with history.replaceState, and sends it as X-Repoview-Token', async () => {
    window.history.replaceState(null, '', `/board?token=${TOKEN}&keep=1#frag`)
    const replaceState = vi.spyOn(window.history, 'replaceState')

    captureToken()

    expect(sessionStorage.getItem(TOKEN_STORAGE_KEY)).toBe(TOKEN)
    expect(replaceState).toHaveBeenCalledTimes(1)
    expect(window.location.search).toBe('?keep=1')
    expect(window.location.href).not.toContain(TOKEN)
    expect(window.location.pathname).toBe('/board')
    expect(window.location.hash).toBe('#frag')

    const fetchMock = stubFetch(() => jsonResponse(allPresent))
    await loadSnapshot()

    const { url, init } = onlyFetch(fetchMock)
    expect(url).toBe('/api/snapshot')
    expect(TOKEN_HEADER).toBe('X-Repoview-Token')
    expect(headerOf(init, 'X-Repoview-Token')).toBe(TOKEN)
  })

  it('leaves the address bar and a stored token alone when the URL carries none', () => {
    sessionStorage.setItem(TOKEN_STORAGE_KEY, TOKEN)
    window.history.replaceState(null, '', '/?keep=1')
    const replaceState = vi.spyOn(window.history, 'replaceState')

    captureToken()

    expect(replaceState).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(TOKEN_STORAGE_KEY)).toBe(TOKEN)
    expect(window.location.search).toBe('?keep=1')
  })
})

describe('startup never throws on a browser API (class of round-2 finding 2)', () => {
  it('a sessionStorage.setItem that throws (quota) keeps the token in memory for the next fetch', async () => {
    window.history.replaceState(null, '', `/?token=${TOKEN}`)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('quota', 'QuotaExceededError')
    })

    expect(() => {
      captureToken()
    }).not.toThrow()
    expect(window.location.href).not.toContain(TOKEN)

    const fetchMock = stubFetch(() => jsonResponse(allPresent))
    await loadSnapshot()
    expect(headerOf(onlyFetch(fetchMock).init, TOKEN_HEADER)).toBe(TOKEN)
  })

  it('a history.replaceState that throws neither breaks startup nor loses the token', async () => {
    window.history.replaceState(null, '', `/?token=${TOKEN}`)
    vi.spyOn(window.history, 'replaceState').mockImplementation(() => {
      throw new DOMException('denied', 'SecurityError')
    })

    expect(() => {
      captureToken()
    }).not.toThrow()

    const fetchMock = stubFetch(() => jsonResponse(allPresent))
    await loadSnapshot()
    expect(headerOf(onlyFetch(fetchMock).init, TOKEN_HEADER)).toBe(TOKEN)
  })
})

describe('mode (acceptance 6)', () => {
  it('server mode: no repoview-mode meta reads /api/snapshot with the stored token', async () => {
    sessionStorage.setItem(TOKEN_STORAGE_KEY, TOKEN)
    const fetchMock = stubFetch(() => jsonResponse(allPresent))

    expect(readMode()).toBe('server')
    const result = await loadSnapshot()

    const { url, init } = onlyFetch(fetchMock)
    expect(url).toBe('/api/snapshot')
    expect(headerOf(init, TOKEN_HEADER)).toBe(TOKEN)
    expect(result).toEqual({ state: 'ready', snapshot: allPresent })
  })

  it('static mode: <meta name="repoview-mode" content="static"> reads ./data/snapshot.json and sends no token', async () => {
    setMode('static')
    sessionStorage.setItem(TOKEN_STORAGE_KEY, TOKEN)
    const fetchMock = stubFetch(() => jsonResponse(allPresent))

    expect(readMode()).toBe('static')
    const result = await loadSnapshot()

    const { url, init } = onlyFetch(fetchMock)
    expect(url).toBe('./data/snapshot.json')
    expect(headerOf(init, TOKEN_HEADER)).toBeNull()
    expect(JSON.stringify(fetchMock.mock.calls)).not.toContain(TOKEN)
    expect(result).toEqual({ state: 'ready', snapshot: allPresent })
  })
})

describe('failures', () => {
  it('a 403 from /api/snapshot is token-rejected', async () => {
    stubFetch(() => new Response('forbidden', { status: 403 }))
    expect(await loadSnapshot()).toEqual({ state: 'token-rejected' })
  })

  it('another HTTP failure is an error naming the status', async () => {
    stubFetch(() => new Response('boom', { status: 500 }))
    const result = await loadSnapshot()
    expect(result.state).toBe('error')
    expect(result.state === 'error' && result.message).toContain('500')
  })

  it('a 403 in static mode is a load error naming 403, not a rejected token', async () => {
    setMode('static')
    stubFetch(() => new Response('forbidden', { status: 403 }))
    expect(await loadSnapshot()).toEqual({
      state: 'error',
      message: 'snapshot unavailable: HTTP 403',
    })
  })

  it('a fetch that rejects is a load error carrying the reason', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => Promise.reject(new TypeError('network down'))),
    )
    const result = await loadSnapshot()
    expect(result.state).toBe('error')
    expect(result.state === 'error' && result.message).toContain('network down')
  })

  it('server mode with no stored token sends no token header', async () => {
    const fetchMock = stubFetch(() => jsonResponse(allPresent))
    await loadSnapshot()
    const { url, init } = onlyFetch(fetchMock)
    expect(url).toBe('/api/snapshot')
    expect(headerOf(init, TOKEN_HEADER)).toBeNull()
  })
})
