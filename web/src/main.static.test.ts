import { flushPromises } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { allPresent } from './__fixtures__/snapshots'

// story:static-export acceptance 4: the entry point mounts the SPA with hash routing when the page
// says it is a static export, so a deep link works on any static host and every request goes to
// ./data/.

function setMode(content: string) {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  const meta = document.createElement('meta')
  meta.name = 'repoview-mode'
  meta.content = content
  document.head.append(meta)
}

function requested(fetchMock: ReturnType<typeof vi.fn<typeof fetch>>): string[] {
  return fetchMock.mock.calls.map(([input]) =>
    typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
  )
}

let fetchMock: ReturnType<typeof vi.fn<typeof fetch>>

beforeEach(() => {
  vi.resetModules()
  document.body.innerHTML = '<div id="app"></div>'
  fetchMock = vi.fn<typeof fetch>((input) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const body = url.endsWith('snapshot.json') || url === '/api/snapshot' ? allPresent : []
    return Promise.resolve(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
    )
  })
  vi.stubGlobal('fetch', fetchMock)
})

afterEach(() => {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  window.history.replaceState(null, '', '/')
  vi.unstubAllGlobals()
})

describe('static mode', () => {
  it('a #/plan deep link opens the Board and every request is under ./data/', async () => {
    setMode('static')
    window.history.replaceState(null, '', '/site/index.html#/plan')
    await import('./main')
    await flushPromises()
    await flushPromises()
    expect(document.querySelector('main h1')?.textContent).toBe('Plan · Board')
    const urls = requested(fetchMock)
    expect(urls).toContain('./data/snapshot.json')
    expect(urls).toContain('./data/plan/board.json')
    for (const url of urls) expect(url.startsWith('./data/'), url).toBe(true)
  })

  it('nav links are hash links, so the host is never asked for a page path', async () => {
    setMode('static')
    window.history.replaceState(null, '', '/site/index.html')
    await import('./main')
    await flushPromises()
    const hrefs = [...document.querySelectorAll('[data-test="nav"] a')].map((a) =>
      a.getAttribute('href'),
    )
    expect(hrefs).toEqual(['#/', '#/plan', '#/specs', '#/quality', '#/repository'])
  })
})

describe('server mode', () => {
  it('keeps path routing', async () => {
    setMode('server')
    window.history.replaceState(null, '', '/plan')
    await import('./main')
    await flushPromises()
    expect(document.querySelector('main h1')?.textContent).toBe('Plan · Board')
    const hrefs = [...document.querySelectorAll('[data-test="nav"] a')].map((a) =>
      a.getAttribute('href'),
    )
    expect(hrefs).toEqual(['/', '/plan', '/specs', '/quality', '/repository'])
  })
})
