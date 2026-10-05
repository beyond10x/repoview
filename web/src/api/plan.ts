// The AEP plan as `aep plan artifact … --format json` prints it, through `/api/plan/*`.
// Statuses, their descriptions and kinds come from that output; nothing here lists them.

import { apiGet, type ApiResult } from './client'

export interface Relation {
  relation: string
  target: string
}

/** What blocks an artifact: `blocked_by` entries in `list`, `board` and `explain`. */
export interface Blocking {
  blocker: string
  type: string
  withholds?: unknown
}

/** One artifact as `list` and `board` print it. */
export interface PlanItem {
  id: string
  kind: string
  status: string
  title: string
  path?: string
  relations: Relation[]
  refs?: unknown[]
  blocked_by?: Blocking[]
  tags?: string[]
}

/** One entry of `board`: a status and its artifacts, with the ladder's description when aep gives one. */
export interface BoardColumn {
  status: string
  description?: string | null
  artifacts: PlanItem[]
}

export interface ScopeEntry {
  path: string
  confidence?: string
}

export interface Finding {
  file?: string | null
  line?: number | null
  category?: string
  severity?: string
  verdict?: string
  origin?: string
  message?: string
}

export interface Outcome {
  reviewed?: string
  outcome?: string
  source?: string
  at?: string
}

/** `show`. */
export interface Artifact extends PlanItem {
  summary?: string | null
  owner?: string | null
  revision?: number
  scope?: ScopeEntry[]
  findings?: Finding[]
  outcomes?: Outcome[]
  body?: string | null
}

/** One entry of `history`. */
export interface HistoryEntry {
  at: string
  actor?: string
  artifact?: string
  kind?: string
  revision: number
  change?: { change?: string; from?: string; to?: string; [key: string]: unknown }
}

export interface Need {
  kind: string
  at_least?: number
  held?: number
}

/** One move `explain` says the artifact made. */
export interface Step {
  from: string
  to: string
  at?: string
  revision?: number
  rested_on?: unknown[]
  on_nothing_recorded?: string
  executor?: string
  correlation?: string
  actor?: string
}

/** `explain`. */
export interface Explain {
  artifact: string
  status?: string
  revision?: number
  blocked_by?: Blocking[]
  reached?: Step[]
  recorded_since?: unknown[]
  /** Planning documents aep could not read, so this answer may be missing records. */
  unreadable?: number
  next?: Array<{ status: string; needs?: Need[] }>
}

/** `validate`. */
export interface Validate {
  problems?: string[]
}

/** What the server answers when `aep` failed (502) or is missing (503). */
export interface ToolFailure {
  tool: string
  exit: number | null
  stderr: string
  stdout?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isRelation(value: unknown): value is Relation {
  return isRecord(value) && typeof value.relation === 'string' && typeof value.target === 'string'
}

function isBlocking(value: unknown): value is Blocking {
  return isRecord(value) && typeof value.blocker === 'string' && typeof value.type === 'string'
}

/** The fields every page relies on, as `list`, `board` and `show` print them. */
function isPlanItem(value: unknown): value is PlanItem {
  return (
    isRecord(value) &&
    typeof value.id === 'string' &&
    typeof value.kind === 'string' &&
    typeof value.status === 'string' &&
    typeof value.title === 'string' &&
    Array.isArray(value.relations) &&
    value.relations.every(isRelation) &&
    (value.blocked_by === undefined ||
      (Array.isArray(value.blocked_by) && value.blocked_by.every(isBlocking))) &&
    (value.tags === undefined ||
      (Array.isArray(value.tags) && value.tags.every((tag) => typeof tag === 'string')))
  )
}

function isColumn(value: unknown): value is BoardColumn {
  return (
    isRecord(value) &&
    typeof value.status === 'string' &&
    Array.isArray(value.artifacts) &&
    value.artifacts.every(isPlanItem)
  )
}

function isArrayOf<T>(check: (item: unknown) => item is T) {
  return (value: unknown): value is T[] => Array.isArray(value) && value.every(check)
}

/** `apiGet(path)`, with a document that is not `T` turned into an error naming the route. */
async function load<T>(path: string, accepts: (data: unknown) => data is T): Promise<ApiResult<T>> {
  const result = await apiGet<unknown>(path)
  if (result.state !== 'ready') return result
  if (!accepts(result.data)) {
    return { state: 'error', status: null, message: `unexpected document from /api/${path}` }
  }
  return { state: 'ready', data: result.data }
}

const isExplain = (value: unknown): value is Explain =>
  isRecord(value) &&
  typeof value.artifact === 'string' &&
  (value.blocked_by === undefined ||
    (Array.isArray(value.blocked_by) && value.blocked_by.every(isBlocking)))

const isHistoryEntry = (value: unknown): value is HistoryEntry =>
  isRecord(value) && typeof value.revision === 'number' && typeof value.at === 'string'

export const loadBoard = () => load('plan/board', isArrayOf(isColumn))
export const loadArtifacts = () => load('plan/artifacts', isArrayOf(isPlanItem))
export const loadValidate = () => load('plan/validate', (v): v is Validate => isRecord(v))
export const loadArtifact = (id: string) =>
  load(`plan/artifacts/${id}`, (v): v is Artifact => isPlanItem(v))
export const loadHistory = (id: string) =>
  load(`plan/artifacts/${id}/history`, isArrayOf(isHistoryEntry))
export const loadExplain = (id: string) => load(`plan/artifacts/${id}/explain`, isExplain)

/** The tool failure an error result carries in its response body, if it carries one. */
export function toolFailure(result: ApiResult<unknown>): ToolFailure | null {
  if (result.state !== 'error' || !('body' in result)) return null
  const body: unknown = result.body
  if (!isRecord(body) || typeof body.tool !== 'string' || typeof body.stderr !== 'string') {
    return null
  }
  const exit = typeof body.exit === 'number' ? body.exit : null
  const failure: ToolFailure = { tool: body.tool, exit, stderr: body.stderr }
  if (typeof body.stdout === 'string') failure.stdout = body.stdout
  return failure
}

/** The text that explains a failed result: aep's stderr when the server passed it on. */
export function failureText(result: ApiResult<unknown>): string {
  if (result.state === 'token-rejected') return 'the server rejected the run token'
  if (result.state !== 'error') return ''
  const failure = toolFailure(result)
  if (failure !== null && failure.stderr.trim() !== '') return failure.stderr
  return result.message
}

function problemsOf(value: unknown): string[] | null {
  if (!isRecord(value) || !Array.isArray(value.problems)) return null
  return value.problems.map((problem) =>
    typeof problem === 'string' ? problem : JSON.stringify(problem),
  )
}

export type Validation =
  { state: 'valid' } | { state: 'problems'; lines: string[] } | { state: 'error'; text: string }

/**
 * `validate` as the Board header shows it. aep exits 1 for an invalid store and prints the
 * problems on stdout, so a failure that carries them is shown as problems, not as an error.
 */
export function validation(result: ApiResult<Validate>): Validation {
  if (result.state === 'ready') {
    const lines = problemsOf(result.data) ?? []
    return lines.length === 0 ? { state: 'valid' } : { state: 'problems', lines }
  }
  const stdout = toolFailure(result)?.stdout
  if (stdout !== undefined) {
    try {
      const lines = problemsOf(JSON.parse(stdout))
      if (lines !== null && lines.length > 0) return { state: 'problems', lines }
    } catch {
      // Not JSON: fall through to the stderr.
    }
  }
  return { state: 'error', text: failureText(result) }
}

/** `columns` with only the artifacts matching `text` (id, title or tag) and `kind`. */
export function filterColumns(columns: BoardColumn[], text: string, kind: string): BoardColumn[] {
  const needle = text.trim().toLowerCase()
  if (needle === '' && kind === '') return columns
  const matches = (artifact: PlanItem) =>
    (kind === '' || artifact.kind === kind) &&
    (needle === '' ||
      [artifact.id, artifact.title, ...(artifact.tags ?? [])].some((field) =>
        field.toLowerCase().includes(needle),
      ))
  return columns.map((column) => ({ ...column, artifacts: column.artifacts.filter(matches) }))
}

/** The kinds present on the board, sorted. */
export function kindsOf(columns: BoardColumn[]): string[] {
  return [...new Set(columns.flatMap((column) => column.artifacts.map((a) => a.kind)))].sort()
}

/** The edges that place an artifact below another in the tree, in the order children are shown. */
export const HIERARCHY = ['designs', 'implements', 'decomposes', 'serves'] as const

export interface TreeNode {
  artifact: PlanItem
  /** The hierarchy relations from this artifact to its parent; empty for a root. */
  via: string[]
  children: TreeNode[]
  /** Shown again below itself; its children are not repeated. */
  cycle: boolean
}

function hierarchyRank(relation: string): number {
  const rank = (HIERARCHY as readonly string[]).indexOf(relation)
  return rank === -1 ? HIERARCHY.length : rank
}

/**
 * The plan as a tree. Children of an artifact are the artifacts whose `serves`, `designs`,
 * `decomposes` or `implements` edge targets it; one reachable from two parents is shown under
 * each. Roots are the visions. Everything not reached from a vision is under `unattached`, from
 * the top of its own chain: an artifact with none of the edges and no children stands alone.
 */
export function buildTree(items: PlanItem[]): { roots: TreeNode[]; unattached: TreeNode[] } {
  const byId = new Map(items.map((item) => [item.id, item]))
  const children = new Map<string, Array<{ item: PlanItem; via: string[] }>>()
  for (const item of items) {
    const parents = new Map<string, string[]>()
    for (const { relation, target } of item.relations) {
      if (!(HIERARCHY as readonly string[]).includes(relation)) continue
      parents.set(target, [...(parents.get(target) ?? []), relation])
    }
    for (const [target, via] of parents) {
      via.sort((a, b) => hierarchyRank(a) - hierarchyRank(b))
      children.set(target, [...(children.get(target) ?? []), { item, via }])
    }
  }
  for (const list of children.values()) {
    list.sort(
      (a, b) =>
        hierarchyRank(a.via[0] ?? '') - hierarchyRank(b.via[0] ?? '') ||
        a.item.kind.localeCompare(b.item.kind) ||
        a.item.id.localeCompare(b.item.id),
    )
  }

  const reached = new Set<string>()
  const grow = (item: PlanItem, via: string[], path: Set<string>): TreeNode => {
    reached.add(item.id)
    if (path.has(item.id)) return { artifact: item, via, children: [], cycle: true }
    const below = new Set(path).add(item.id)
    return {
      artifact: item,
      via,
      children: (children.get(item.id) ?? []).map((child) => grow(child.item, child.via, below)),
      cycle: false,
    }
  }

  const roots = items
    .filter((item) => item.kind === 'vision')
    .map((item) => grow(item, [], new Set()))

  const hasParentInStore = (item: PlanItem) =>
    item.relations.some(
      ({ relation, target }) =>
        (HIERARCHY as readonly string[]).includes(relation) && byId.has(target),
    )
  const unattached: TreeNode[] = []
  for (const item of items) {
    if (!reached.has(item.id) && !hasParentInStore(item)) unattached.push(grow(item, [], new Set()))
  }
  // Whatever is left hangs in a cycle that reaches no top; show each from itself.
  for (const item of items) {
    if (!reached.has(item.id)) unattached.push(grow(item, [], new Set()))
  }
  return { roots, unattached }
}

/** Every relation of another artifact that targets `id`, in list order. */
export function incomingRelations(
  items: PlanItem[],
  id: string,
): Array<{ relation: string; source: string }> {
  return items.flatMap((item) =>
    item.relations
      .filter((relation) => relation.target === id)
      .map((relation) => ({ relation: relation.relation, source: item.id })),
  )
}

/** History by revision, then time, oldest first. */
export function oldestFirst(entries: HistoryEntry[]): HistoryEntry[] {
  return [...entries].sort((a, b) => a.revision - b.revision || a.at.localeCompare(b.at))
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

/** Who carried out a move, as aep's text output appends it: `, executed by X, correlation Y`. */
function executedBy(fields: Record<string, unknown>): string {
  const parts: string[] = []
  const executor = text(fields.executor)
  const correlation = text(fields.correlation)
  if (executor !== null) parts.push(`executed by ${executor}`)
  if (correlation !== null) parts.push(`correlation ${correlation}`)
  return parts.map((part) => `, ${part}`).join('')
}

/** What `explain` prints in brackets after a move: revision, actor, executor, correlation. */
export function stepDetails(step: Step): string {
  const parts: string[] = []
  if (typeof step.revision === 'number') parts.push(`revision ${String(step.revision)}`)
  const actor = text(step.actor)
  if (actor !== null) parts.push(`by ${actor}`)
  return parts.join(', ') + executedBy(step as unknown as Record<string, unknown>)
}

/** `kind from source (reference): review was outcome` — the part every evidence line shares. */
function recorded(record: Record<string, unknown>, verb: string): string {
  let line = [text(record.kind), text(verb)].filter((part) => part !== null).join(' ')
  const source = text(record.source)
  if (source !== null) line += `${line === '' ? '' : ' '}from ${source}`
  const reference = text(record.reference)
  if (reference !== null) line += ` (${reference})`
  const review = text(record.review)
  const outcome = text(record.outcome)
  if (review !== null && outcome !== null) line += `: ${review} was ${outcome}`
  else if (outcome !== null) line += `: ${outcome}`
  return line
}

/**
 * One evidence record (`rested_on`, `recorded_since`) as `explain` prints it: kind, source,
 * reference, review outcome, when it was observed and the revision that admitted it. A record
 * of no known shape is shown as its JSON.
 */
export function evidenceText(record: unknown): string {
  if (typeof record === 'string') return record
  if (!isRecord(record)) return JSON.stringify(record)
  let line = recorded(record, '')
  if (line === '') return JSON.stringify(record)
  const at = text(record.at)
  if (at !== null) line += `, observed ${at}`
  if (typeof record.revision === 'number')
    line += `, admitted at revision ${String(record.revision)}`
  return line
}

/** One `history` entry's change as `history` prints it. */
export function historyText(entry: HistoryEntry): string {
  const change = entry.change
  if (change === undefined) return ''
  const name = text(change.change) ?? 'changed'
  if (name === 'moved' && typeof change.from === 'string' && typeof change.to === 'string') {
    return `moved ${change.from} → ${change.to}${executedBy(change)}`
  }
  if (name === 'evidence') return recorded(change, 'recorded')
  const rest = Object.fromEntries(Object.entries(change).filter(([key]) => key !== 'change'))
  return Object.keys(rest).length === 0 ? name : `${name} ${JSON.stringify(rest)}`
}
