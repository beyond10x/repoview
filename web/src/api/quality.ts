// Wire contract: story:quality-codegate acceptance 2. The server names the beyond10x codegate it
// found on PATH, its version, every `codegate` it passed over and the commands `codegate --help`
// lists, and says why there is no assessment. No Codegate release assesses source yet, so the
// page shows no score, rating or finding; `assessment` is typed `unknown` and never rendered.

import { apiGet } from './client'
import { TOKEN_REJECTED_MESSAGE } from './snapshot'

/** Where the beyond10x Codegate lives. */
export const CODEGATE_REPOSITORY = 'https://github.com/beyond10x/codegate'

/** The contract's 503 text, shown when the server sent none of its own. */
export const NOT_FOUND_MESSAGE = 'beyond10x codegate not found on PATH'

export interface QualityReport {
  tool: string
  tool_path: string
  tool_version: string
  /** Every `codegate` on PATH before `tool_path` that is not the beyond10x one. */
  skipped: string[]
  /** The subcommands `codegate --help` lists, as the server read them. */
  commands: string[]
  /** `null` until a Codegate release has a source assessment; never rendered. */
  assessment: unknown
  /** Why there is no assessment. */
  reason: string | null
}

export type QualityState =
  | { state: 'loading' }
  | { state: 'ready'; report: QualityReport }
  /** The server found no beyond10x codegate (503). */
  | { state: 'unavailable'; message: string }
  | { state: 'error'; status: number | null; message: string }

/** A JSON object (not `null`, not an array). */
function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isStrings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((entry: unknown) => typeof entry === 'string')
}

function isReport(value: unknown): value is QualityReport {
  return (
    isRecord(value) &&
    typeof value.tool === 'string' &&
    typeof value.tool_path === 'string' &&
    typeof value.tool_version === 'string' &&
    isStrings(value.skipped) &&
    isStrings(value.commands) &&
    (value.reason === null || typeof value.reason === 'string')
  )
}

/** The `stderr` of a tool-error body, when it has non-blank text. */
function stderrOf(body: unknown): string | null {
  const stderr = isRecord(body) ? body.stderr : undefined
  return typeof stderr === 'string' && stderr.trim() !== '' ? stderr : null
}

/** One read of `/api/quality`. Always settles to a state; never rejects. */
export async function loadQuality(): Promise<QualityState> {
  const result = await apiGet<unknown>('quality')
  switch (result.state) {
    case 'ready':
      return isReport(result.data)
        ? { state: 'ready', report: result.data }
        : { state: 'error', status: null, message: 'unexpected /api/quality document' }
    case 'token-rejected':
      return { state: 'error', status: 403, message: TOKEN_REJECTED_MESSAGE }
    case 'error': {
      const stderr = stderrOf(result.body)
      if (result.status === 503) {
        return { state: 'unavailable', message: stderr ?? NOT_FOUND_MESSAGE }
      }
      const detail = stderr === null ? '' : `: ${stderr}`
      return {
        state: 'error',
        status: result.status,
        message: `quality unavailable: ${result.message}${detail}`,
      }
    }
  }
}
