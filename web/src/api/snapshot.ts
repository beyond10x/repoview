// Wire contract: story:server-skeleton § Wire contract.

export type SourceKind = 'Vcs' | 'Planning' | 'Specification' | 'Quality' | 'Documents'

export type Availability = 'Present' | 'Absent' | 'ToolMissing' | 'Failed'

export interface Source {
  source_id: string
  kind: SourceKind
  location: string
  availability: Availability
  tool: string | null
  tool_path: string | null
  tool_version: string | null
  diagnostic: string | null
  summary: Record<string, unknown>
}

export interface Snapshot {
  repoview_version: string
  project: { root: string; name: string }
  sources: Source[]
}

export type SnapshotState =
  | { state: 'loading' }
  | { state: 'ready'; snapshot: Snapshot }
  | { state: 'token-rejected' }
  | { state: 'error'; message: string }

export type Mode = 'server' | 'static'

export const TOKEN_STORAGE_KEY = 'repoview.token'
export const TOKEN_HEADER = 'X-Repoview-Token'

export const TOKEN_REJECTED_MESSAGE = 'token rejected — reopen the URL repoview printed'

const API_SNAPSHOT_URL = '/api/snapshot'
const STATIC_SNAPSHOT_URL = './data/snapshot.json'

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
 * Moves `?token=` from the address bar into sessionStorage (story:server-skeleton § Token
 * transport). Never throws: it runs before the app mounts, and a throw there is a blank page.
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

/** Always settles to a state; never rejects. */
export async function loadSnapshot(): Promise<SnapshotState> {
  try {
    const isStatic = readMode() === 'static'
    const headers: Record<string, string> = {}
    const token = isStatic ? null : storedToken()
    if (token !== null) headers[TOKEN_HEADER] = token
    const response = await fetch(isStatic ? STATIC_SNAPSHOT_URL : API_SNAPSHOT_URL, { headers })
    if (response.status === 403 && !isStatic) return { state: 'token-rejected' }
    if (!response.ok) {
      return { state: 'error', message: `snapshot unavailable: HTTP ${String(response.status)}` }
    }
    return { state: 'ready', snapshot: (await response.json()) as Snapshot }
  } catch (error) {
    return { state: 'error', message: `snapshot unavailable: ${String(error)}` }
  }
}
