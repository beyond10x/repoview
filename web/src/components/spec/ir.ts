// Reading the compiled ESS IR for display: short names, anchors, type labels, and the lifecycle
// of an entity as a Mermaid stateDiagram-v2.

import type { Assignment, CommandDecl, Condition, Entity, Instance, TypeRef } from '../../api/spec'

/** The last `.`-separated segment of a qualified name. */
export function shortName(name: string): string {
  const at = name.lastIndexOf('.')
  return at === -1 ? name : name.slice(at + 1)
}

/** The element id of an entity's card. */
export function entityAnchor(name: string): string {
  return `entity-${name}`
}

/** The element id of any other declaration's card (`command`, `event`, `view`, …). */
export function declAnchor(kind: string, name: string): string {
  return `${kind}-${name}`
}

/** A type reference as a reader writes it: `string`, `Money`, `string?`, `list<LineItem>`. */
export function typeLabel(ref: TypeRef | undefined): string {
  if (ref === undefined) return 'unknown'
  switch (ref.kind) {
    case 'primitive':
      return ref.name ?? 'primitive'
    case 'declared':
      return ref.name === undefined ? 'declared' : shortName(ref.name)
    case 'optional':
      return `${typeLabel(ref.of)}?`
    case 'list':
    case 'set':
      return `${ref.kind}<${typeLabel(ref.of)}>`
    case 'map': {
      const key =
        typeof ref.key === 'string'
          ? ref.key
          : typeof ref.key === 'object' && ref.key !== null
            ? typeLabel(ref.key as TypeRef)
            : 'unknown'
      return `map<${key}, ${typeLabel(ref.value)}>`
    }
    default:
      return ref.name === undefined ? ref.kind : `${ref.kind} ${ref.name}`
  }
}

/**
 * Text a diagram statement can carry: letters, digits, space and `_ . , ( ) / -` only. Anything
 * else (quotes, `;`, `%%`, `:`, markup, line breaks) becomes a space, so a name from the
 * specification can never end its statement or start another.
 */
function label(text: string): string {
  const plain = text
    .replace(/[^\p{L}\p{N} _.,()/-]/gu, ' ')
    .replace(/\s+/g, ' ')
    .trim()
  return plain === '' ? '?' : plain
}

/** The commands with an outcome on `entity` with `effect`, and the transition each moves by. */
function subjects(
  entity: string,
  commands: Record<string, CommandDecl>,
): { command: string; effect: string; transition: string | null }[] {
  const found: { command: string; effect: string; transition: string | null }[] = []
  for (const command of Object.values(commands)) {
    for (const outcome of command.outcomes) {
      const subject = outcome.subject
      if (subject?.entity !== entity) continue
      found.push({
        command: shortName(command.name),
        effect: subject.effect,
        transition: subject.transition?.name ?? null,
      })
    }
  }
  return found
}

function unique(items: string[]): string[] {
  return [...new Set(items)]
}

/**
 * The entity's lifecycle as a Mermaid `stateDiagram-v2`: every state declared with its name as
 * the label, `[*]` into the initial state (labelled with the commands that create the entity),
 * one edge per transition and source state (labelled with the commands that move it, then the
 * transition's name), and an edge out of each terminal state. `null` without a lifecycle or
 * without any state; a lifecycle with no initial state has no `[*]` edge into it.
 */
export function lifecycleSource(
  entity: Entity,
  commands: Record<string, CommandDecl>,
): string | null {
  const lifecycle = entity.lifecycle
  if (lifecycle === undefined) return null
  const transitions = lifecycle.transitions
  const ids = new Map<string, string>()
  const lines = ['stateDiagram-v2']
  const id = (state: string): string => {
    let known = ids.get(state)
    if (known === undefined) {
      known = `s${String(ids.size)}`
      ids.set(state, known)
      lines.push(`    state "${label(state)}" as ${known}`)
    }
    return known
  }
  for (const state of lifecycle.states) id(state)
  for (const transition of transitions) {
    for (const from of transition.from) id(from)
    id(transition.to)
  }
  if (lifecycle.initial !== undefined) id(lifecycle.initial)
  for (const terminal of lifecycle.terminal) id(terminal)
  if (ids.size === 0) return null

  const acting = subjects(entity.name, commands)
  const creators = unique(acting.filter((s) => s.effect === 'creates').map((s) => s.command))
  if (lifecycle.initial !== undefined) {
    const initial = `    [*] --> ${id(lifecycle.initial)}`
    lines.push(creators.length > 0 ? `${initial}: ${label(creators.join(', '))}` : initial)
  }
  for (const transition of transitions) {
    const movers = unique(
      acting.filter((s) => s.transition === transition.name).map((s) => s.command),
    )
    const text = movers.length > 0 ? `${movers.join(', ')} (${transition.name})` : transition.name
    for (const from of transition.from) {
      lines.push(`    ${id(from)} --> ${id(transition.to)}: ${label(text)}`)
    }
  }
  for (const terminal of lifecycle.terminal) lines.push(`    ${id(terminal)} --> [*]`)
  return lines.join('\n')
}

/**
 * The condition an outcome is taken on, in words: `when <predicate>` for `when`, `when <cause>`
 * for `external`, `otherwise`, `when the entity is in the wrong state`; any other kind as its
 * name, with its predicate or cause when ess gave one.
 */
export function conditionText(condition: Condition): string {
  const detail = condition.predicate ?? condition.cause
  switch (condition.kind) {
    case 'otherwise':
      return 'otherwise'
    case 'wrong_state':
      return 'when the entity is in the wrong state'
    case 'when':
    case 'external':
      return detail === undefined ? `when ${condition.kind}` : `when ${detail}`
    default: {
      const kind = condition.kind.replace(/_/g, ' ')
      return detail === undefined ? kind : `${kind}: ${detail}`
    }
  }
}

/** Which instance an outcome acts on: `the supplied invoice_id`, `observed from Event.field`. */
export function instanceText(instance: Instance): string {
  const field = instance.field?.name
  if (instance.from === 'supplied' && field !== undefined) return `the supplied ${field}`
  if (instance.from === 'observed' && instance.event !== undefined) {
    return `observed from ${shortName(instance.event)}${field === undefined ? '' : `.${field}`}`
  }
  return field === undefined ? instance.from : `${instance.from} ${field}`
}

/** One assignment: `target ← input_field` or `target ← literal`. */
export function assignmentText(assignment: Assignment): string {
  const value = assignment.value
  const from = value.field ?? value.value ?? value.kind.replace(/_/g, ' ')
  return `${assignment.target} ← ${from}`
}
