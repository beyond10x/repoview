// The `/api/spec/*` wire contract (story:spec-pages acceptance 1) and the parts of ess 0.52.0's
// output the Specs pages read. The server passes `ess specify` output through unchanged, so the
// IR and the graph go through `normalizeIr` / `normalizeGraph` before any page sees them: every
// array below is an array, every name a string, and what ess left out is `undefined` where the
// type says so. The pages render a missing identity, lifecycle or outcome list as "none".

import { apiGet, type ApiResult } from './client'

/** One of the three per-root documents. */
export type View = 'ir' | 'graph' | 'mermaid'

export interface Naming {
  summary?: string
  display?: string
  wire?: string
}

export interface TypeRef {
  kind: string
  name?: string
  of?: TypeRef
  key?: unknown
  value?: TypeRef
}

export interface Field {
  name: string
  /** `undefined` when ess gave no type; shown as "unknown". */
  type_ref?: TypeRef
  naming?: Naming
}

export interface Transition {
  name: string
  from: string[]
  to: string
}

export interface Lifecycle {
  states: string[]
  initial?: string
  terminal: string[]
  transitions: Transition[]
}

export interface Relation {
  name: string
  kind: string
  /** The target entity's name; `undefined` when ess named none. */
  target?: string
  cardinality?: string
  via?: string
}

export interface Entity {
  name: string
  domain: string
  identity?: Field
  fields: Field[]
  lifecycle?: Lifecycle
  invariants: unknown[]
  relations: Relation[]
  naming?: Naming
}

/** How a command finds the instance an outcome acts on. */
export interface Instance {
  /** `supplied` (an input field names it) or `observed` (an emitted event's field does). */
  from: string
  /** The event whose field names the instance, when `observed`. */
  event?: string
  field?: Field
}

export interface Subject {
  entity: string
  effect: string
  transition?: Transition
  instance?: Instance
}

/** One field an outcome sets on the entity, from an input field or a literal. */
export interface Assignment {
  target: string
  value: { kind: string; field?: string; value?: string }
}

export interface Condition {
  /** `when`, `otherwise`, `external`, `wrong_state`, … as ess writes it. */
  kind: string
  /** The cause of an `external` condition. */
  cause?: string
  /** The expression of a `when` condition. */
  predicate?: string
}

export interface Outcome {
  name: string
  condition?: Condition
  subject?: Subject
  emits: string[]
  sets: Assignment[]
  /** True when the outcome answers with the command's response. */
  returns?: boolean
  error?: string
  summary?: string
}

export interface CommandDecl {
  name: string
  domain: string
  input: Field[]
  response: Field[]
  outcomes: Outcome[]
  naming?: Naming
}

export interface EventDecl {
  name: string
  domain: string
  fields: Field[]
  naming?: Naming
}

export interface ViewDecl {
  name: string
  domain: string
  source?: string
  fields: Field[]
  consistency?: string
  /** The expression rows are selected by, as ess writes it. */
  filter?: string
  /** Sort keys, e.g. `issued_at desc`. */
  order_by: string[]
  naming?: Naming
}

export interface ComponentDecl {
  name: string
  owns: string[]
  accepts: string[]
  publishes: string[]
  /** How other components reach it, e.g. `network`. */
  reached_by?: string
  naming?: Naming
}

export interface DomainDecl {
  name: string
  naming?: Naming
  types: string[]
  entities: string[]
  commands: string[]
  events: string[]
  errors: string[]
  views: string[]
  actors: string[]
}

/** `ess specify compile --format json`, normalised. */
export interface Ir {
  system: string
  version: string
  domains: Record<string, DomainDecl>
  entities: Record<string, Entity>
  commands: Record<string, CommandDecl>
  events: Record<string, EventDecl>
  views: Record<string, ViewDecl>
  components: Record<string, ComponentDecl>
}

export interface GraphNode {
  kind: string
  name: string
  domain?: string
}

export interface GraphEdge {
  kind: string
  from: string
  to: string
  label?: string
}

/** `ess specify graph --format json`, normalised. */
export interface Graph {
  system: string
  version: string
  groups: { kind: string; label: string; members: string[] }[]
  nodes: GraphNode[]
  edges: GraphEdge[]
}

type Raw = Record<string, unknown>

function rec(value: unknown): Raw {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? (value as Raw) : {}
}

function list(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function text(value: unknown): string | undefined {
  return typeof value === 'string' ? value : typeof value === 'number' ? String(value) : undefined
}

function texts(value: unknown): string[] {
  return list(value).flatMap((item) => {
    const found = text(item)
    return found === undefined ? [] : [found]
  })
}

function naming(value: unknown): Naming | undefined {
  if (typeof value !== 'object' || value === null) return undefined
  const raw = rec(value)
  const out: Naming = {}
  const summary = text(raw.summary)
  const display = text(raw.display)
  const wire = text(raw.wire)
  if (summary !== undefined) out.summary = summary
  if (display !== undefined) out.display = display
  if (wire !== undefined) out.wire = wire
  return out
}

function typeRef(value: unknown): TypeRef | undefined {
  const raw = rec(value)
  const kind = text(raw.kind)
  if (kind === undefined) return undefined
  const out: TypeRef = { kind }
  const name = text(raw.name)
  if (name !== undefined) out.name = name
  const of = typeRef(raw.of)
  if (of !== undefined) out.of = of
  if (raw.key !== undefined && raw.key !== null) out.key = raw.key
  const inner = typeRef(raw.value)
  if (inner !== undefined) out.value = inner
  return out
}

function field(value: unknown, index: number): Field {
  const raw = rec(value)
  const out: Field = { name: text(raw.name) ?? `field ${String(index + 1)}` }
  const ref = typeRef(raw.type_ref)
  if (ref !== undefined) out.type_ref = ref
  const named = naming(raw.naming)
  if (named !== undefined) out.naming = named
  return out
}

function fields(value: unknown): Field[] {
  return list(value).map(field)
}

function transition(value: unknown, index: number): Transition | null {
  const raw = rec(value)
  const to = text(raw.to)
  if (to === undefined) return null
  return { name: text(raw.name) ?? `transition ${String(index + 1)}`, from: texts(raw.from), to }
}

function lifecycle(value: unknown): Lifecycle | undefined {
  if (typeof value !== 'object' || value === null) return undefined
  const raw = rec(value)
  const out: Lifecycle = {
    states: texts(raw.states),
    terminal: texts(raw.terminal),
    transitions: list(raw.transitions).flatMap((item, index) => transition(item, index) ?? []),
  }
  const initial = text(raw.initial)
  if (initial !== undefined) out.initial = initial
  return out
}

function relation(value: unknown, index: number): Relation {
  const raw = rec(value)
  const out: Relation = {
    name: text(raw.name) ?? `relation ${String(index + 1)}`,
    kind: text(raw.kind) ?? 'relation',
  }
  const target = text(raw.target)
  const cardinality = text(raw.cardinality)
  const via = text(raw.via)
  if (target !== undefined) out.target = target
  if (cardinality !== undefined) out.cardinality = cardinality
  if (via !== undefined) out.via = via
  return out
}

function instance(value: unknown): Instance | undefined {
  if (typeof value !== 'object' || value === null) return undefined
  const raw = rec(value)
  const out: Instance = { from: text(raw.from) ?? 'unknown' }
  const event = text(raw.event)
  if (event !== undefined) out.event = event
  if (typeof raw.field === 'object' && raw.field !== null) out.field = field(raw.field, 0)
  return out
}

function subject(value: unknown): Subject | undefined {
  const raw = rec(value)
  const entity = text(raw.entity)
  if (entity === undefined) return undefined
  const out: Subject = { entity, effect: text(raw.effect) ?? 'affects' }
  const moved = typeof raw.transition === 'object' ? transition(raw.transition, 0) : null
  if (moved !== null) out.transition = moved
  const found = instance(raw.instance)
  if (found !== undefined) out.instance = found
  return out
}

function assignment(value: unknown, index: number): Assignment {
  const raw = rec(value)
  const given = rec(raw.value)
  const out: Assignment = {
    target: text(raw.target) ?? `field ${String(index + 1)}`,
    value: { kind: text(given.kind) ?? 'value' },
  }
  const from = text(given.field)
  const literal = text(given.value)
  if (from !== undefined) out.value.field = from
  if (literal !== undefined) out.value.value = literal
  return out
}

function condition(value: unknown): Condition | undefined {
  const raw = rec(value)
  const kind = text(raw.kind)
  if (kind === undefined) return undefined
  const out: Condition = { kind }
  const cause = text(raw.cause)
  const predicate = text(raw.predicate)
  if (cause !== undefined) out.cause = cause
  if (predicate !== undefined) out.predicate = predicate
  return out
}

function outcome(value: unknown, index: number): Outcome {
  const raw = rec(value)
  const out: Outcome = {
    name: text(raw.name) ?? `outcome ${String(index + 1)}`,
    emits: texts(raw.emits),
    sets: list(raw.sets).map(assignment),
  }
  const taken = condition(raw.condition)
  if (taken !== undefined) out.condition = taken
  const acted = subject(raw.subject)
  if (acted !== undefined) out.subject = acted
  if (typeof raw.returns === 'boolean') out.returns = raw.returns
  const error = text(raw.error)
  const summary = text(raw.summary)
  if (error !== undefined) out.error = error
  if (summary !== undefined) out.summary = summary
  return out
}

/** `record`'s entries, each named by its own `name` or else by its key. */
function declarations<T>(record: unknown, build: (raw: Raw, name: string) => T): Record<string, T> {
  const out: Record<string, T> = {}
  for (const [key, value] of Object.entries(rec(record))) {
    const raw = rec(value)
    out[key] = build(raw, text(raw.name) ?? key)
  }
  return out
}

function withNaming<T extends { naming?: Naming }>(decl: T, raw: Raw): T {
  const named = naming(raw.naming)
  if (named !== undefined) decl.naming = named
  return decl
}

/** Any JSON as an `Ir`: what ess left out or mistyped becomes empty or `undefined`, never a throw. */
export function normalizeIr(value: unknown): Ir {
  const raw = rec(value)
  return {
    system: text(raw.system) ?? 'unnamed system',
    version: text(raw.version) ?? 'unversioned',
    domains: declarations<DomainDecl>(raw.domains, (d, name) =>
      withNaming<DomainDecl>(
        {
          name,
          types: texts(d.types),
          entities: texts(d.entities),
          commands: texts(d.commands),
          events: texts(d.events),
          errors: texts(d.errors),
          views: texts(d.views),
          actors: texts(d.actors),
        },
        d,
      ),
    ),
    entities: declarations<Entity>(raw.entities, (e, name) => {
      const entity: Entity = {
        name,
        domain: text(e.domain) ?? '',
        fields: fields(e.fields),
        invariants: list(e.invariants),
        relations: list(e.relations).map(relation),
      }
      if (typeof e.identity === 'object' && e.identity !== null) {
        entity.identity = field(e.identity, 0)
      }
      const cycle = lifecycle(e.lifecycle)
      if (cycle !== undefined) entity.lifecycle = cycle
      return withNaming(entity, e)
    }),
    commands: declarations<CommandDecl>(raw.commands, (c, name) =>
      withNaming<CommandDecl>(
        {
          name,
          domain: text(c.domain) ?? '',
          input: fields(c.input),
          response: fields(c.response),
          outcomes: list(c.outcomes).map(outcome),
        },
        c,
      ),
    ),
    events: declarations<EventDecl>(raw.events, (e, name) =>
      withNaming<EventDecl>({ name, domain: text(e.domain) ?? '', fields: fields(e.fields) }, e),
    ),
    views: declarations<ViewDecl>(raw.views, (v, name) => {
      const view: ViewDecl = {
        name,
        domain: text(v.domain) ?? '',
        fields: fields(v.fields),
        order_by: texts(v.order_by),
      }
      const source = text(v.source)
      const consistency = text(v.consistency)
      const filter = text(v.filter)
      if (source !== undefined) view.source = source
      if (consistency !== undefined) view.consistency = consistency
      if (filter !== undefined) view.filter = filter
      return withNaming(view, v)
    }),
    components: declarations<ComponentDecl>(raw.components, (c, name) => {
      const component: ComponentDecl = {
        name,
        owns: texts(c.owns),
        accepts: texts(c.accepts),
        publishes: texts(c.publishes),
      }
      const reached = text(c.reached_by)
      if (reached !== undefined) component.reached_by = reached
      return withNaming(component, c)
    }),
  }
}

/** Any JSON as a `Graph`: nodes without a name and edges without both ends are dropped. */
export function normalizeGraph(value: unknown): Graph {
  const raw = rec(value)
  return {
    system: text(raw.system) ?? 'unnamed system',
    version: text(raw.version) ?? 'unversioned',
    groups: list(raw.groups).map((item) => {
      const group = rec(item)
      return {
        kind: text(group.kind) ?? 'group',
        label: text(group.label) ?? '',
        members: texts(group.members),
      }
    }),
    nodes: list(raw.nodes).flatMap((item) => {
      const node = rec(item)
      const name = text(node.name)
      if (name === undefined) return []
      const out: GraphNode = { kind: text(node.kind) ?? 'node', name }
      const domain = text(node.domain)
      if (domain !== undefined) out.domain = domain
      return [out]
    }),
    edges: list(raw.edges).flatMap((item) => {
      const edge = rec(item)
      const from = text(edge.from)
      const to = text(edge.to)
      if (from === undefined || to === undefined) return []
      const out: GraphEdge = { kind: text(edge.kind) ?? 'edge', from, to }
      const label = text(edge.label)
      if (label !== undefined) out.label = label
      return [out]
    }),
  }
}

/** One entry of `/api/spec/roots`. `validate` is `ess specify validate` JSON, or the tool failure. */
export interface RootEntry {
  root: string
  validate: Record<string, unknown>
  ok: boolean
}

/**
 * How the project root `.` is spelt in a URL, in the API path and the page route alike: a browser
 * drops `.` and `%2e` segments before sending, so `.` can never be a key. The server reads `~` as
 * `.` (`crates/repoview/src/api/spec.rs`, `ROOT_KEY`).
 */
export const ROOT_KEY = '~'

/** A root as its URL key: `.` is [`ROOT_KEY`], every other root is itself. */
export function rootKey(root: string): string {
  return root === '.' ? ROOT_KEY : root
}

/** The API path of one root's document, e.g. `spec/roots/crates/a/ess/ir` or `spec/roots/~/ir`. */
export function rootApiPath(root: string, view: View): string {
  return `spec/roots/${rootKey(root)}/${view}`
}

/** The page path of a root, `/` kept and every segment URL-encoded: `/specs/crates/a/ess`. */
export function rootPagePath(root: string): string {
  return `/specs/${rootKey(root).split('/').map(encodeURIComponent).join('/')}`
}

/** The root a `/specs/:root(.*)` param names; `~` is `.`. */
export function rootFromParam(param: string | string[]): string {
  const key = Array.isArray(param) ? param.join('/') : param
  return key === ROOT_KEY ? '.' : key
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isRootEntry(value: unknown): value is RootEntry {
  return (
    isRecord(value) &&
    typeof value.root === 'string' &&
    typeof value.ok === 'boolean' &&
    isRecord(value.validate)
  )
}

function shapeError<T>(what: string): ApiResult<T> {
  return { state: 'error', status: null, message: `unexpected ${what} answer from the server` }
}

export async function loadRoots(): Promise<ApiResult<RootEntry[]>> {
  const result = await apiGet<unknown>('spec/roots')
  if (result.state !== 'ready') return result
  return Array.isArray(result.data) && result.data.every(isRootEntry)
    ? { state: 'ready', data: result.data }
    : shapeError('spec/roots')
}

export async function loadIr(root: string): Promise<ApiResult<Ir>> {
  const result = await apiGet<unknown>(rootApiPath(root, 'ir'))
  if (result.state !== 'ready') return result
  return isRecord(result.data)
    ? { state: 'ready', data: normalizeIr(result.data) }
    : shapeError('IR')
}

export async function loadGraph(root: string): Promise<ApiResult<Graph>> {
  const result = await apiGet<unknown>(rootApiPath(root, 'graph'))
  if (result.state !== 'ready') return result
  return isRecord(result.data)
    ? { state: 'ready', data: normalizeGraph(result.data) }
    : shapeError('graph')
}

/** The Mermaid text of the interaction graph. */
export async function loadMermaid(root: string): Promise<ApiResult<string>> {
  const result = await apiGet<unknown>(rootApiPath(root, 'mermaid'))
  if (result.state !== 'ready') return result
  return isRecord(result.data) && typeof result.data.mermaid === 'string'
    ? { state: 'ready', data: result.data.mermaid }
    : shapeError('mermaid')
}

function strings(value: unknown): string[] {
  return Array.isArray(value)
    ? value.map((item) => (typeof item === 'string' ? item : JSON.stringify(item)))
    : []
}

function textLines(value: unknown): string[] {
  return typeof value === 'string' ? value.split('\n').filter((line) => line.trim() !== '') : []
}

/**
 * Why a validate result refuses the root, one line each and verbatim: `problems` when ess gave
 * them, else the tool failure's exit, stderr and stdout, else `unresolved_references`. A valid
 * result has none.
 */
export function refusalLines(validate: Record<string, unknown>): string[] {
  const problems = strings(validate.problems)
  if (problems.length > 0) return problems
  if (typeof validate.tool === 'string') {
    const exit = validate.exit
    const head =
      typeof exit === 'number'
        ? `${validate.tool} exited with ${String(exit)}`
        : `${validate.tool} failed`
    return [head, ...textLines(validate.stderr), ...textLines(validate.stdout)]
  }
  if (validate.valid === true) return []
  const unresolved = strings(validate.unresolved_references)
  if (unresolved.length > 0) return unresolved.map((name) => `unresolved reference: ${name}`)
  return ['ess did not report the specification as valid']
}
