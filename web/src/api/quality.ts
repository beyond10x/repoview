// Wire contract: story:quality-page acceptance 2. The assessment is codegate's own document,
// passed through by the server. Every field of it, at any depth, is typed `unknown`: it may be
// absent, `null` or of another type, so it is read only through the accessors below, and the
// type checker refuses a read that skips them.

import { apiGet } from './client'
import { TOKEN_REJECTED_MESSAGE } from './snapshot'

export type LanguageStatus = 'assessed' | 'not-assessed' | 'failed' | 'running'

/** codegate's assessment document, unchecked. */
export type Assessment = Readonly<Record<string, unknown>>

/** One entry of an assessment's `top_findings`, unchecked. */
export type Finding = Readonly<Record<string, unknown>>

export interface LanguageQuality {
  language: string
  status: LanguageStatus
  reason: string | null
  stderr?: string | null
  assessment: Assessment | null
}

export interface QualityReport {
  tool: string
  tool_path: string
  languages: LanguageQuality[]
}

/** How often the page asks again while an assessment is running. */
export const POLL_INTERVAL_MS = 2000

export type QualityState =
  | { state: 'loading' }
  | { state: 'ready'; report: QualityReport }
  /** The server has no assessing tool (503). */
  | { state: 'unavailable'; message: string }
  | { state: 'error'; status: number | null; message: string }

function isReport(value: unknown): value is QualityReport {
  if (!isRecord(value)) return false
  const languages = value.languages
  return (
    Array.isArray(languages) &&
    languages.every(
      (entry: unknown) =>
        isRecord(entry) && typeof entry.language === 'string' && typeof entry.status === 'string',
    )
  )
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
    case 'error':
      // The contract's 503: no assessing tool on the server's PATH.
      if (result.status === 503) {
        return { state: 'unavailable', message: 'codegate not found on PATH — nothing assessed' }
      }
      return {
        state: 'error',
        status: result.status,
        message: `quality unavailable: ${result.message}`,
      }
  }
}

export function isRunning(report: QualityReport): boolean {
  return report.languages.some((entry) => entry.status === 'running')
}

// Accessors: the only way the page reads the pass-through document.

/** A JSON object (not `null`, not an array). */
export function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** `value[key]` when `value` is a JSON object, else `undefined`. */
function field(value: unknown, key: string): unknown {
  return isRecord(value) ? value[key] : undefined
}

/** Non-blank text, else `null`. */
function text(value: unknown): string | null {
  return typeof value === 'string' && value.trim() !== '' ? value : null
}

function finite(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function numberEntries(value: unknown): Array<[string, number]> {
  if (!isRecord(value)) return []
  return Object.entries(value).flatMap(([key, entry]): Array<[string, number]> => {
    const number = finite(entry)
    return number === null ? [] : [[key, number]]
  })
}

export function rating(assessment: Assessment): string | null {
  return text(assessment.rating)
}

/** `score_max` when it is a positive number, else `null`. */
export function scoreMax(assessment: Assessment): number | null {
  const max = finite(assessment.score_max)
  return max !== null && max > 0 ? max : null
}

export interface ScoreRow {
  name: string
  value: number
  /** `value / max`, clamped to 0…1; `null` when the assessment gives no usable `score_max`. */
  fraction: number | null
}

/** The numeric scores, `overall` first, then in document order. */
export function scoreRows(assessment: Assessment): ScoreRow[] {
  const max = scoreMax(assessment)
  const rows = numberEntries(assessment.scores).map(([name, value]) => ({
    name,
    value,
    fraction: max === null ? null : Math.min(1, Math.max(0, value / max)),
  }))
  return [
    ...rows.filter((row) => row.name === 'overall'),
    ...rows.filter((r) => r.name !== 'overall'),
  ]
}

/** The numeric finding counts, largest first. */
export function countRows(assessment: Assessment): Array<{ kind: string; count: number }> {
  return numberEntries(assessment.finding_counts)
    .map(([kind, count]) => ({ kind, count }))
    .sort((a, b) => b.count - a.count || a.kind.localeCompare(b.kind))
}

/** The summary's scalar entries, in document order. */
export function summaryRows(assessment: Assessment): Array<[string, string]> {
  const summary = assessment.summary
  if (!isRecord(summary)) return []
  return Object.entries(summary).flatMap(([key, value]): Array<[string, string]> =>
    typeof value === 'number' || typeof value === 'string' || typeof value === 'boolean'
      ? [[key, String(value)]]
      : [],
  )
}

/** The entries of `top_findings` that are objects; anything else is skipped. */
export function topFindings(assessment: Assessment): Finding[] {
  const findings = assessment.top_findings
  return Array.isArray(findings) ? findings.filter(isRecord) : []
}

/** What a finding says: its own title, else its reason or message, else its kind. */
export function findingTitle(finding: Finding): string {
  for (const key of ['title', 'reason', 'message', 'kind']) {
    const found = text(finding[key])
    if (found !== null) return found
  }
  return 'untitled finding'
}

export function findingSeverity(finding: Finding): string | null {
  return text(finding.severity)
}

/** `uri:line` with codegate's 0-based line shown 1-based, `uri` alone without a line. */
export function findingLocation(finding: Finding): string | null {
  const location = finding.location
  if (typeof location === 'string') return text(location)
  const uri = text(field(location, 'uri'))
  if (uri === null) return null
  const line = finite(field(field(field(location, 'range'), 'start'), 'line'))
  return line === null ? uri : `${uri}:${String(line + 1)}`
}

/** `snake_case` keys as words. */
export function humanise(key: string): string {
  return key.replaceAll('_', ' ')
}

/** A colour band for a letter rating: its first letter, or `none`. */
export function ratingBand(rating: string | null): 'a' | 'b' | 'c' | 'd' | 'none' {
  const letter = rating?.trim().charAt(0).toLowerCase()
  if (letter === 'a' || letter === 'b' || letter === 'c') return letter
  if (letter === 'd' || letter === 'e' || letter === 'f') return 'd'
  return 'none'
}

/** A colour band for a score bar. */
export function scoreBand(fraction: number | null): 'good' | 'fair' | 'poor' | 'none' {
  if (fraction === null) return 'none'
  if (fraction >= 0.8) return 'good'
  if (fraction >= 0.5) return 'fair'
  return 'poor'
}
