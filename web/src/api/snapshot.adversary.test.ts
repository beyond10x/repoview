import { afterEach, describe, expect, it, vi } from 'vitest'
import { TOKEN_HEADER, captureToken, loadSnapshot } from './snapshot'

// With site storage blocked (Chrome "Block all cookies", Firefox cookieBehavior 2), reading
// `window.sessionStorage` throws a SecurityError. `captureToken()` runs at the top of main.ts before
// `createApp`, so a throw there leaves a blank page with no message; `loadSnapshot()` reads storage
// outside its try, so it rejects and `useSnapshot()` (no catch) stays at "loading snapshot…".
const TOKEN = 'b'.repeat(64)

function blockStorage() {
  vi.spyOn(window, 'sessionStorage', 'get').mockImplementation(() => {
    throw new DOMException('Access is denied for this document.', 'SecurityError')
  })
}

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  window.history.replaceState(null, '', '/')
})

describe('adversary pass 2: site storage blocked', () => {
  it('captureToken() does not throw, still strips ?token, and the token still reaches /api/snapshot', async () => {
    window.history.replaceState(null, '', `/?token=${TOKEN}`)
    blockStorage()

    expect(() => {
      captureToken()
    }).not.toThrow()
    expect(window.location.href).not.toContain(TOKEN)

    const fetchMock = vi.fn<typeof fetch>(() =>
      Promise.resolve(new Response('{}', { status: 200 })),
    )
    vi.stubGlobal('fetch', fetchMock)
    await loadSnapshot()
    expect(new Headers(fetchMock.mock.calls[0]?.[1]?.headers).get(TOKEN_HEADER)).toBe(TOKEN)
  })

  it('loadSnapshot() settles to a state instead of rejecting', async () => {
    blockStorage()
    vi.stubGlobal(
      'fetch',
      vi.fn<typeof fetch>(() => Promise.resolve(new Response('{}', { status: 200 }))),
    )
    await expect(loadSnapshot()).resolves.toHaveProperty('state')
  })
})
