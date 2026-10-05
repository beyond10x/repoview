// The one way the SPA reads data: `/api/<path>` with the run token from the server, or
// `./data/<path>.json` in a static export. Token transport: story:server-skeleton § Token transport.

export type Mode = 'server' | 'static'

export type ApiResult<T> =
  | { state: 'ready'; data: T }
  | { state: 'token-rejected' }
  | { state: 'error'; status: number | null; message: string }

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

/** `path` with each `/`-separated segment URL-encoded. */
function encodePath(path: string): string {
  return path.split('/').map(encodeURIComponent).join('/')
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
    const url = isStatic ? `./data/${encodePath(path)}.json` : `/api/${encodePath(path)}`
    const response = await fetch(url, { headers })
    if (response.status === 403 && !isStatic) return { state: 'token-rejected' }
    if (!response.ok) {
      return { state: 'error', status: response.status, message: `HTTP ${String(response.status)}` }
    }
    return { state: 'ready', data: (await response.json()) as T }
  } catch (error) {
    return { state: 'error', status: null, message: String(error) }
  }
}
