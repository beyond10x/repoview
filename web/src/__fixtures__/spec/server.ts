// A stand-in for the server in the Specs page tests: answers `/api/snapshot` and `/api/spec/*`
// from the captured fixtures, and records every URL fetched.
import { vi } from 'vitest'
import { allPresent } from '../snapshots'
import { CAPTURED, rootsAnswer } from './index'

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

/** The URL a `fetch` input names. */
export function requestUrl(input: RequestInfo | URL): string {
  return typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
}

/** Path → response body (status 200), or a `Response` for anything else. */
export type Routes = Record<string, unknown>

/** The routes for every captured root plus the refused one. */
export function capturedRoutes(): Routes {
  const routes: Routes = {
    '/api/snapshot': allPresent,
    '/api/spec/roots': rootsAnswer(),
  }
  for (const c of CAPTURED) {
    const base = `/api/spec/roots/${c.root === '.' ? '~' : c.root}`
    routes[`${base}/ir`] = c.rawIr
    routes[`${base}/graph`] = c.rawGraph
    routes[`${base}/mermaid`] = { mermaid: c.mermaid }
  }
  return routes
}

export function serve(routes: Routes): void {
  vi.stubGlobal(
    'fetch',
    vi.fn<typeof fetch>((input) => {
      const path = new URL(requestUrl(input), 'http://127.0.0.1:7480').pathname
      const answer = routes[path]
      if (answer instanceof Response) return Promise.resolve(answer)
      if (answer === undefined) return Promise.resolve(json({ error: 'not found' }, 404))
      return Promise.resolve(json(answer))
    }),
  )
}

export function fetched(): string[] {
  return vi
    .mocked(fetch)
    .mock.calls.map(([input]) => new URL(requestUrl(input), 'http://127.0.0.1:7480').pathname)
}

export { json }
