import { vi } from 'vitest'

/**
 * Each URL's answer: a JSON body served with status 200, or `{ status, body? }` (an object with
 * only those keys and a numeric `status`) for any other status.
 */
export type Routes = Record<string, unknown>

function isStatus(answer: unknown): answer is { status: number; body?: unknown } {
  return (
    typeof answer === 'object' &&
    answer !== null &&
    'status' in answer &&
    typeof answer.status === 'number' &&
    Object.keys(answer).every((key) => key === 'status' || key === 'body')
  )
}

/**
 * Stubs `fetch` to answer each URL in `routes` (e.g. `/api/vcs`); any other URL is 404. Returns the
 * mock, so a test can read which URLs were requested.
 */
export function serveRoutes(routes: Routes) {
  const mock = vi.fn<typeof fetch>((input) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
    const answer = routes[url]
    if (answer === undefined) return Promise.resolve(new Response('not found\n', { status: 404 }))
    const { status, body } = isStatus(answer) ? answer : { status: 200, body: answer }
    return Promise.resolve(
      new Response(body === undefined ? '' : JSON.stringify(body), {
        status,
        headers: { 'content-type': 'application/json' },
      }),
    )
  })
  vi.stubGlobal('fetch', mock)
  return mock
}

export function requestedUrls(mock: ReturnType<typeof serveRoutes>): string[] {
  return mock.mock.calls.map(([input]) =>
    typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
  )
}
