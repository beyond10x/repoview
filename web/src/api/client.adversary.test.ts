import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import { createAppRouter } from '../router'
import { TOKEN_STORAGE_KEY, apiGet, forgetToken } from './client'

// Adversary, story:page-frame acceptance 2: "path segments URL-encoded, `/` kept". encodeURIComponent
// leaves `.` and `..` unchanged, and fetch resolves them as dot-segments, so an id that is `..`
// names a different document, outside the prefix the caller asked for.

const ORIGIN = 'http://127.0.0.1:7480'

function setMode(content: string | null) {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  if (content === null) return
  const meta = document.createElement('meta')
  meta.name = 'repoview-mode'
  meta.content = content
  document.head.append(meta)
}

/** The path fetch would actually request, resolved the way the browser resolves it; null if none. */
async function requestedPath(path: string): Promise<string | null> {
  const fetchMock = vi.fn<typeof fetch>(() => Promise.resolve(new Response('{}', { status: 200 })))
  vi.stubGlobal('fetch', fetchMock)
  await apiGet(path)
  const call = fetchMock.mock.calls[0]
  if (call === undefined) return null
  const input = call[0]
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  return new URL(url, `${ORIGIN}/`).pathname
}

beforeEach(() => {
  forgetToken()
  sessionStorage.clear()
  sessionStorage.setItem(TOKEN_STORAGE_KEY, 'c'.repeat(64))
})

afterEach(() => {
  setMode(null)
  vi.unstubAllGlobals()
})

describe('apiGet never lets an id segment leave the prefix it was put under', () => {
  it('the router hands a spec root of "../../snapshot" to its page, decoded', async () => {
    // What reaches apiGet: the /specs/:root(.*) route this story added.
    const router = createAppRouter(createMemoryHistory())
    await router.push('/specs/..%2F..%2Fsnapshot')
    expect(router.currentRoute.value.params.root).toBe('../../snapshot')
  })

  for (const mode of ['server', 'static'] as const) {
    const prefix = mode === 'server' ? '/api/spec/roots/' : '/data/spec/roots/'

    it(`${mode} mode: a "..", "." or "../.." id stays under ${prefix} or is not fetched`, async () => {
      setMode(mode)
      for (const id of ['..', '.', '../..', '../../snapshot']) {
        const path = await requestedPath(`spec/roots/${id}/ir`)
        if (path !== null) {
          expect(path, `id ${JSON.stringify(id)} requested ${path}`).toMatch(
            new RegExp(`^${prefix.replace(/\//g, '\\/')}`),
          )
        }
      }
    })
  }
})
