import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { apiGet, forgetExportManifest } from './client'
import { loadArtifact, loadExplain, loadHistory, loadValidate, validation } from './plan'
import { loadDocument } from './repository'
import { loadIr, loadMermaid } from './spec'

// story:static-export acceptances 2 and 4: in a static export every page reads
// `./data/<api path>.json`; a route `./data/export.json` lists as not 2xx is read from
// `./data/<api path>.error.json`, holding `{ status, body }`, whatever the host answers for the
// `.json` the export never wrote.

function setStatic() {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  const meta = document.createElement('meta')
  meta.name = 'repoview-mode'
  meta.content = 'static'
  document.head.append(meta)
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

/** A static host holding exactly `files`; anything else is what `missing` answers (404). */
function host(
  files: Record<string, unknown>,
  missing: () => Response = () => new Response('not found', { status: 404 }),
) {
  const fetchMock = vi.fn<typeof fetch>((input) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const file = Object.hasOwn(files, url) ? files[url] : undefined
    return Promise.resolve(file === undefined ? missing() : json(file))
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

function urls(fetchMock: ReturnType<typeof host>): string[] {
  return fetchMock.mock.calls.map(([input]) =>
    typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
  )
}

function manifest(routes: Array<[string, number]>) {
  return {
    repoview_version: '0.1.0',
    exported_at: '2026-10-05T00:00:00Z',
    routes: routes.map(([path, status]) => ({ path, status })),
    files: [],
  }
}

const AEP_FAILURE = { tool: 'aep', exit: 1, stderr: '', stdout: '{"problems":["story:x is bad"]}' }

beforeEach(() => {
  forgetExportManifest()
  setStatic()
})

afterEach(() => {
  document.head.querySelector('meta[name="repoview-mode"]')?.remove()
  vi.unstubAllGlobals()
})

describe('static mode reads ./data/<api path>.json for every page request', () => {
  it('artifact, history and explain of an id with a colon', async () => {
    const fetchMock = host({})
    await loadArtifact('story:x')
    await loadHistory('story:x')
    await loadExplain('story:x')
    expect(urls(fetchMock).filter((url) => url !== './data/export.json')).toEqual([
      './data/plan/artifacts/story%3Ax.json',
      './data/plan/artifacts/story%3Ax/history.json',
      './data/plan/artifacts/story%3Ax/explain.json',
    ])
  })

  it('the project root is spelt ~ and a nested root keeps its slashes', async () => {
    const fetchMock = host({})
    await loadIr('.')
    await loadMermaid('crates/a/ess')
    expect(urls(fetchMock).filter((url) => url !== './data/export.json')).toEqual([
      './data/spec/roots/~/ir.json',
      './data/spec/roots/crates/a/ess/mermaid.json',
    ])
  })

  it('a document is one request', async () => {
    const fetchMock = host({ './data/docs/README.md.json': { name: 'README.md', markdown: '# x' } })
    expect(await loadDocument('README.md')).toEqual({
      state: 'ready',
      data: { name: 'README.md', markdown: '# x' },
    })
    expect(urls(fetchMock)).toEqual(['./data/docs/README.md.json'])
  })
})

describe('a route export.json lists as not 2xx answers its .error.json', () => {
  const files = {
    './data/export.json': manifest([
      ['plan/validate', 502],
      ['docs/CHANGELOG.md', 413],
      ['quality', 200],
    ]),
    './data/plan/validate.error.json': { status: 502, body: AEP_FAILURE },
    './data/docs/CHANGELOG.md.error.json': { status: 413, body: 'too large' },
  }

  it('with its status and body', async () => {
    const fetchMock = host(files)
    expect(await apiGet('plan/validate')).toEqual({
      state: 'error',
      status: 502,
      message: 'HTTP 502',
      body: AEP_FAILURE,
    })
    expect(urls(fetchMock)).toEqual([
      './data/plan/validate.json',
      './data/export.json',
      './data/plan/validate.error.json',
    ])
  })

  it('directly, once export.json has been read', async () => {
    host(files)
    await apiGet('quality')
    const fetchMock = host(files)
    expect(await apiGet('plan/validate')).toMatchObject({ state: 'error', status: 502 })
    expect(urls(fetchMock)).toEqual(['./data/plan/validate.error.json'])
  })

  for (const [what, missing] of [
    ['403', () => new Response('forbidden', { status: 403 })],
    ['a 200 HTML fallback page', () => new Response('<!doctype html>', { status: 200 })],
    ['500', () => new Response('no', { status: 500 })],
  ] as const) {
    it(`whatever the host answers for the missing .json: ${what}`, async () => {
      host(files, missing)
      expect(await apiGet('plan/validate')).toEqual({
        state: 'error',
        status: 502,
        message: 'HTTP 502',
        body: AEP_FAILURE,
      })
    })
  }

  it('so the Board still shows the problems aep printed', async () => {
    host(files)
    expect(validation(await loadValidate())).toEqual({
      state: 'problems',
      lines: ['story:x is bad'],
    })
  })

  it('a document that answered 413 keeps its own message', async () => {
    host(files)
    expect(await loadDocument('CHANGELOG.md')).toEqual({
      state: 'unavailable',
      reason: 'error',
      message: 'CHANGELOG.md is larger than 1 MiB and is not shown',
    })
  })

  it('a route listed 2xx or not listed is never read from an error file', async () => {
    const fetchMock = host(files)
    expect(await apiGet('quality')).toEqual({ state: 'error', status: 404, message: 'HTTP 404' })
    expect(await apiGet('vcs')).toEqual({ state: 'error', status: 404, message: 'HTTP 404' })
    expect(urls(fetchMock).filter((url) => url.endsWith('.error.json'))).toEqual([])
  })

  it('an error file without a numeric status is ignored: the host answer stands', async () => {
    host({
      './data/export.json': manifest([['vcs', 404]]),
      './data/vcs.error.json': { body: {} },
    })
    expect(await apiGet('vcs')).toEqual({ state: 'error', status: 404, message: 'HTTP 404' })
  })

  it('without a readable export.json the host answer stands, and it is read again next time', async () => {
    const fetchMock = host({
      './data/plan/validate.error.json': { status: 502, body: AEP_FAILURE },
    })
    expect(await apiGet('plan/validate')).toEqual({
      state: 'error',
      status: 404,
      message: 'HTTP 404',
    })
    await apiGet('plan/validate')
    expect(urls(fetchMock).filter((url) => url === './data/export.json')).toHaveLength(2)
  })
})

describe('server mode never reads export.json or an error file', () => {
  it('a 404 is a 404 after one request', async () => {
    document.head.querySelector('meta[name="repoview-mode"]')?.remove()
    const fetchMock = host({})
    expect(await apiGet('quality')).toEqual({ state: 'error', status: 404, message: 'HTTP 404' })
    expect(urls(fetchMock)).toEqual(['/api/quality'])
  })
})
