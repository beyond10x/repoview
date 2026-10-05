// The one way the SPA reads data: `/api/<path>` with the run token from the server, or
// `./data/<path>.json` in a static export. Token transport: story:server-skeleton § Token transport.

export type Mode = 'server' | 'static'

// An error's `body` is the response's JSON when it had one, e.g. a tool's stderr behind a 502.
export type ApiResult<T> =
  | { state: 'ready'; data: T }
  | { state: 'token-rejected' }
  | { state: 'error'; status: number | null; message: string; body?: unknown }

export const TOKEN_STORAGE_KEY = 'repoview.token'
export const TOKEN_HEADER = 'X-Repoview-Token'

// Used only when sessionStorage refuses the token (site data blocked, quota): the token then lives
// for this page load, which is all the API needs.
let memoryToken: string | null = null

function storeToken(token: string): void {
  try {
    window.sessionStorage.setItem(TOKEN_STORAGE_KEY, token)
  } catch {
    memoryToken = token
  }
}

/** Drops the token from both sessionStorage and memory. */
export function forgetToken(): void {
  memoryToken = null
  try {
    window.sessionStorage.removeItem(TOKEN_STORAGE_KEY)
  } catch {
    // Storage blocked: nothing stored there.
  }
}

function storedToken(): string | null {
  let stored: string | null = null
  try {
    stored = window.sessionStorage.getItem(TOKEN_STORAGE_KEY)
  } catch {
    // Storage blocked: fall through to the in-memory copy.
  }
  return stored ?? memoryToken
}

/**
 * Moves `?token=` from the address bar into sessionStorage. Never throws: it runs before the app
 * mounts, and a throw there is a blank page.
 */
export function captureToken(): void {
  const url = new URL(window.location.href)
  const token = url.searchParams.get('token')
  if (token === null) return
  storeToken(token)
  url.searchParams.delete('token')
  try {
    window.history.replaceState(window.history.state, '', url.pathname + url.search + url.hash)
  } catch {
    // The token stays visible in the address bar; the app still works.
  }
}

export function readMode(): Mode {
  const meta = document.querySelector<HTMLMetaElement>('meta[name="repoview-mode"]')
  return meta?.content === 'static' ? 'static' : 'server'
}

/** `segment` percent-decoded, or as given when it holds a malformed escape. */
function decoded(segment: string): string {
  try {
    return decodeURIComponent(segment)
  } catch {
    return segment
  }
}

/**
 * `path` with each `/`-separated segment URL-encoded, or `null` when a segment is empty, `.` or
 * `..` after decoding: the URL parser would resolve those, and the request would leave its prefix.
 */
function encodePath(path: string): string | null {
  const segments = path.split('/')
  if (segments.some((segment) => ['', '.', '..'].includes(decoded(segment)))) return null
  return segments.map(encodeURIComponent).join('/')
}

/** The response body parsed as JSON, or `undefined` when it is not JSON. */
async function jsonOrNothing(response: Response): Promise<unknown> {
  try {
    return (await response.json()) as unknown
  } catch {
    return undefined
  }
}

/**
 * GET one API document, e.g. `apiGet('plan/board')`. Always settles to a result; never rejects.
 * Only a 403 from the server is `token-rejected`; in static mode there is no token to reject.
 */
export async function apiGet<T>(path: string): Promise<ApiResult<T>> {
  try {
    const isStatic = readMode() === 'static'
    const headers: Record<string, string> = {}
    const token = isStatic ? null : storedToken()
    if (token !== null) headers[TOKEN_HEADER] = token
    const encoded = encodePath(path)
    if (encoded === null) {
      return { state: 'error', status: null, message: `refused path ${JSON.stringify(path)}` }
    }
    const url = isStatic ? `./data/${encoded}.json` : `/api/${encoded}`
    const response = await fetch(url, { headers })
    if (response.status === 403 && !isStatic) return { state: 'token-rejected' }
    if (!response.ok) {
      const message = `HTTP ${String(response.status)}`
      const body = await jsonOrNothing(response)
      if (body === undefined) return { state: 'error', status: response.status, message }
      return { state: 'error', status: response.status, message, body }
    }
    return { state: 'ready', data: (await response.json()) as T }
  } catch (error) {
    return { state: 'error', status: null, message: String(error) }
  }
}
