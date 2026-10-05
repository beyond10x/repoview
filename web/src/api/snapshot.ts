// Wire contract: story:server-skeleton § Wire contract.

import { apiGet } from './client'

export {
  TOKEN_HEADER,
  TOKEN_STORAGE_KEY,
  captureToken,
  forgetToken,
  readMode,
  type Mode,
} from './client'

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

export const TOKEN_REJECTED_MESSAGE = 'token rejected — reopen the URL repoview printed'

/** Always settles to a state; never rejects. */
export async function loadSnapshot(): Promise<SnapshotState> {
  const result = await apiGet<Snapshot>('snapshot')
  switch (result.state) {
    case 'ready':
      return { state: 'ready', snapshot: result.data }
    case 'token-rejected':
      return result
    case 'error':
      return { state: 'error', message: `snapshot unavailable: ${result.message}` }
  }
}
