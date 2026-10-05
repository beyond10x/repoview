import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { CAPTURED, refused } from '../__fixtures__/spec'
import { requestUrl } from '../__fixtures__/spec/server'
import { TOKEN_HEADER, TOKEN_STORAGE_KEY } from './client'
import {
  loadGraph,
  loadIr,
  loadMermaid,
  loadRoots,
  normalizeGraph,
  normalizeIr,
  refusalLines,
  rootApiPath,
  ROOT_KEY,
  rootFromParam,
  rootPagePath,
} from './spec'

// story:spec-pages: the client side of the `/api/spec/*` wire contract.

function okJson(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

beforeEach(() => {
  sessionStorage.clear()
  sessionStorage.setItem(TOKEN_STORAGE_KEY, 't0ken')
  vi.stubGlobal(
    'fetch',
    vi.fn<typeof fetch>(() => Promise.resolve(okJson({}))),
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
})

function fetchedUrls(): string[] {
  return vi.mocked(fetch).mock.calls.map(([input]) => requestUrl(input))
}

describe('root paths', () => {
  it('a root keeps its slashes in the API path', () => {
    expect(rootApiPath('ess', 'ir')).toBe('spec/roots/ess/ir')
    expect(rootApiPath('crates/a/ess', 'mermaid')).toBe('spec/roots/crates/a/ess/mermaid')
  })

  it('the project root `.` is `~` (coordinator addition): never a dot segment on the wire', () => {
    expect(ROOT_KEY).toBe('~')
    expect(rootApiPath('.', 'graph')).toBe('spec/roots/~/graph')
    for (const view of ['ir', 'graph', 'mermaid'] as const) {
      expect(rootApiPath('.', view).split('/')).not.toContain('.')
    }
  })

  it('the page path of `.` is /specs/~; other roots keep their slashes, segments encoded', () => {
    expect(rootPagePath('.')).toBe('/specs/~')
    expect(rootPagePath('crates/a/ess')).toBe('/specs/crates/a/ess')
    expect(rootPagePath('a b/#x')).toBe('/specs/a%20b/%23x')
    expect(rootFromParam('~')).toBe('.')
    expect(rootFromParam('crates/a/ess')).toBe('crates/a/ess')
    expect(rootFromParam(['crates', 'a'])).toBe('crates/a')
  })
})

describe('loaders', () => {
  it('fetch the four routes with the token', async () => {
    await loadRoots()
    await loadIr('crates/a/ess')
    await loadGraph('.')
    await loadMermaid('ess')
    expect(fetchedUrls()).toEqual([
      '/api/spec/roots',
      '/api/spec/roots/crates/a/ess/ir',
      '/api/spec/roots/~/graph',
      '/api/spec/roots/ess/mermaid',
    ])
    for (const [, init] of vi.mocked(fetch).mock.calls) {
      expect((init?.headers as Record<string, string>)[TOKEN_HEADER]).toBe('t0ken')
    }
  })

  it('a roots answer that is not an array is an error, never a crash', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ sources: [] }))
    const result = await loadRoots()
    expect(result.state).toBe('error')
  })

  it('a mermaid answer without a string is an error', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ nope: 1 }))
    const result = await loadMermaid('ess')
    expect(result.state).toBe('error')
  })
})

describe('refusalLines', () => {
  it('are the validate problems, verbatim', () => {
    const lines = refusalLines(refused.validate)
    expect(lines).toEqual(refused.validate.problems)
    expect(lines[0]).toContain('[undeclared_reference]')
  })

  it('a tool failure gives its stderr and stdout lines', () => {
    expect(refusalLines({ tool: 'ess', exit: 9, stderr: 'nope\nworse\n', stdout: 'out' })).toEqual([
      'ess exited with 9',
      'nope',
      'worse',
      'out',
    ])
  })

  it('a valid result has none', () => {
    expect(refusalLines({ valid: true, system: 's' })).toEqual([])
  })

  it('valid: false with no problems still says why it is refused', () => {
    expect(
      refusalLines({ valid: false, unresolved_references: ['a.B', 'c.D'] }).join('\n'),
    ).toContain('a.B')
    expect(refusalLines({ valid: false })).not.toEqual([])
  })
})

describe('normalizeIr', () => {
  type Raw = Record<string, Record<string, Record<string, unknown>>>

  it('loses nothing a page shows from real ess output', () => {
    for (const c of CAPTURED) {
      const raw = c.rawIr as Raw
      const ir = normalizeIr(c.rawIr)
      for (const kind of ['domains', 'entities', 'commands', 'events', 'views', 'components']) {
        expect(Object.keys(ir[kind as keyof typeof ir]), `${c.root} ${kind}`).toEqual(
          Object.keys(raw[kind] ?? {}),
        )
      }
      for (const [name, entity] of Object.entries(ir.entities)) {
        const source = raw.entities?.[name] ?? {}
        expect(entity.lifecycle, name).toEqual(
          source.lifecycle === undefined
            ? undefined
            : { transitions: [], ...(source.lifecycle as object) },
        )
        expect(entity.identity?.name, name).toBe((source.identity as { name: string }).name)
        expect(
          entity.fields.map((f) => f.name),
          name,
        ).toEqual((source.fields as { name: string }[]).map((f) => f.name))
        expect(entity.relations, name).toEqual(source.relations ?? [])
      }
      for (const [name, command] of Object.entries(ir.commands)) {
        const source = raw.commands?.[name] ?? {}
        expect(command.outcomes.length, name).toBe((source.outcomes as unknown[]).length)
      }
    }
  })

  it('lifecycle: null is no lifecycle', () => {
    const ir = normalizeIr({ entities: { 'a.E': { name: 'a.E', lifecycle: null } } })
    expect(ir.entities['a.E']?.lifecycle).toBeUndefined()
  })

  it('a command without outcomes has none', () => {
    const ir = normalizeIr({ commands: { 'a.C': { name: 'a.C' } } })
    expect(ir.commands['a.C']?.outcomes).toEqual([])
    expect(ir.commands['a.C']?.input).toEqual([])
  })

  it('an entity without identity has none, and keeps its key as its name', () => {
    const ir = normalizeIr({ entities: { 'a.E': { fields: null } } })
    expect(ir.entities['a.E']?.identity).toBeUndefined()
    expect(ir.entities['a.E']?.name).toBe('a.E')
    expect(ir.entities['a.E']?.fields).toEqual([])
  })

  it('anything that is not an IR object normalises to an empty IR, never a throw', () => {
    for (const value of [null, 1, 'x', [], { entities: [1, 2] }, { commands: 'no' }]) {
      const ir = normalizeIr(value)
      expect(Object.keys(ir.entities)).toEqual(value === null ? [] : Object.keys(ir.entities))
      expect(typeof ir.system).toBe('string')
    }
  })
})

describe('normalizeGraph', () => {
  it('keeps what the page shows of real ess output; drops nameless nodes and one-ended edges', () => {
    type RawGraph = {
      groups: unknown[]
      nodes: unknown[]
      edges: { kind: string; from: string; to: string; label?: string }[]
    }
    for (const c of CAPTURED) {
      const raw = c.rawGraph as RawGraph
      const graph = normalizeGraph(c.rawGraph)
      expect(graph.groups, c.root).toEqual(raw.groups)
      expect(graph.nodes, c.root).toEqual(raw.nodes)
      // An edge's `delivery` and `on_failure` (ess 0.52.0 policy edges) are not shown.
      expect(graph.edges, c.root).toEqual(
        raw.edges.map(({ kind, from, to, label }) =>
          label === undefined ? { kind, from, to } : { kind, from, to, label },
        ),
      )
    }
    const graph = normalizeGraph({
      nodes: [{ kind: 'command' }],
      edges: [{ from: 'a' }],
      groups: null,
    })
    expect(graph.nodes).toEqual([])
    expect(graph.edges).toEqual([])
    expect(graph.groups).toEqual([])
  })
})
