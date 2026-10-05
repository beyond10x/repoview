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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

const isSuccess = (status: number) => status >= 200 && status < 300

// A static export's `./data/export.json` routes, `path → status`, once read on this page load.
let exportRoutes: Map<string, number> | null = null

/** Drops the cached `./data/export.json` routes (tests; a page load starts without them). */
export function forgetExportManifest(): void {
  exportRoutes = null
}

/**
 * The status of every exported route, from `./data/export.json` (`repoview export`), or `null`
 * when it cannot be read. Read once; a failed read is tried again next time.
 */
async function exportStatuses(): Promise<Map<string, number> | null> {
  if (exportRoutes !== null) return exportRoutes
  try {
    const response = await fetch('./data/export.json')
    if (!response.ok) return null
    const manifest = await jsonOrNothing(response)
    if (!isRecord(manifest) || !Array.isArray(manifest.routes)) return null
    const routes = new Map<string, number>()
    for (const route of manifest.routes as unknown[]) {
      if (isRecord(route) && typeof route.path === 'string' && typeof route.status === 'number') {
        routes.set(route.path, route.status)
      }
    }
    exportRoutes = routes
    return routes
  } catch {
    return null
  }
}

/**
 * The error a static export recorded for `encoded`: `repoview export` writes an answer that was
 * not 2xx as `./data/<path>.error.json` holding `{ status, body }`. `null` when there is none.
 */
async function exportedError(encoded: string): Promise<ApiResult<never> | null> {
  try {
    const response = await fetch(`./data/${encoded}.error.json`)
    if (!response.ok) return null
    const recorded = await jsonOrNothing(response)
    if (!isRecord(recorded) || typeof recorded.status !== 'number') return null
    const { status } = recorded
    const message = `HTTP ${String(status)}`
    if (recorded.body === undefined) return { state: 'error', status, message }
    return { state: 'error', status, message, body: recorded.body }
  } catch {
    return null
  }
}

/**
 * One document of a static export. A route `export.json` lists as not 2xx is read from its
 * `.error.json`, whatever the host answers for the `.json` it never wrote: directly once the
 * manifest has been read, else after the `.json` read gave no document. A successful read costs
 * one request.
 */
async function staticGet<T>(path: string, encoded: string): Promise<ApiResult<T>> {
  const known = exportRoutes?.get(path)
  if (known !== undefined && !isSuccess(known)) {
    const exported = await exportedError(encoded)
    if (exported !== null) return exported
  }
  const response = await fetch(`./data/${encoded}.json`, { headers: {} })
  let parsed: { data: unknown } | { failure: string }
  if (response.ok) {
    try {
      parsed = { data: JSON.parse(await response.text()) as unknown }
    } catch (error) {
      parsed = { failure: String(error) }
    }
    if ('data' in parsed) return { state: 'ready', data: parsed.data as T }
  } else {
    parsed = { failure: `HTTP ${String(response.status)}` }
  }
  if (known === undefined) {
    const status = (await exportStatuses())?.get(path)
    if (status !== undefined && !isSuccess(status)) {
      const exported = await exportedError(encoded)
      if (exported !== null) return exported
    }
  }
  if (response.ok) return { state: 'error', status: null, message: parsed.failure }
  const message = parsed.failure
  const body = await jsonOrNothing(response)
  if (body === undefined) return { state: 'error', status: response.status, message }
  return { state: 'error', status: response.status, message, body }
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
    if (isStatic) return await staticGet<T>(path, encoded)
    const response = await fetch(`/api/${encoded}`, { headers })
    if (response.status === 403) return { state: 'token-rejected' }
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
