import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import artifactsFixture from '../__fixtures__/plan/artifacts.json'
import blockedItem from '../__fixtures__/plan/blocked-item.json'
import explainBlocked from '../__fixtures__/plan/explain-blocked.json'
import explainExecuted from '../__fixtures__/plan/explain-executed.json'
import historyExecuted from '../__fixtures__/plan/history-executed.json'
import historyReview from '../__fixtures__/plan/history-review.json'
import historyFixture from '../__fixtures__/plan/history.json'
import { failedWith } from '../__fixtures__/plan/failures'
import {
  buildTree,
  evidenceText,
  filterColumns,
  historyText,
  incomingRelations,
  kindsOf,
  loadArtifact,
  loadArtifacts,
  loadBoard,
  loadExplain,
  loadHistory,
  loadValidate,
  oldestFirst,
  stepDetails,
  toolFailure,
  validation,
  type BoardColumn,
  type Explain,
  type HistoryEntry,
  type PlanItem,
  type TreeNode,
} from './plan'

// story:plan-pages acceptances 2, 3, 4 and 5: the data side of the three pages.

const artifacts = artifactsFixture as PlanItem[]

function item(id: string, relations: Array<[string, string]> = [], kind?: string): PlanItem {
  return {
    id,
    kind: kind ?? id.split(':')[0] ?? 'unknown',
    status: 'draft',
    title: `title of ${id}`,
    relations: relations.map(([relation, target]) => ({ relation, target })),
  }
}

/** Each node as `id` with its children indented below it, for whole-tree comparisons. */
function outline(nodes: TreeNode[], depth = 0): string[] {
  return nodes.flatMap((node) => [
    `${'  '.repeat(depth)}${node.artifact.id}${node.cycle ? ' (cycle)' : ''}`,
    ...outline(node.children, depth + 1),
  ])
}

function childIds(node: TreeNode | undefined): string[] {
  return (node?.children ?? []).map((child) => child.artifact.id)
}

function find(nodes: TreeNode[], id: string): TreeNode | undefined {
  return nodes.find((node) => node.artifact.id === id)
}

describe('buildTree over this repository store (acceptance 3)', () => {
  const tree = buildTree(artifacts)
  const vision = find(tree.roots, 'vision:repoview')
  const design = find(vision?.children ?? [], 'architecture-design:repoview')

  it('the roots are the visions', () => {
    expect(tree.roots.map((node) => node.artifact.id)).toEqual(['vision:repoview'])
  })

  it('a design sits under the vision it designs', () => {
    expect(design?.via).toEqual(['designs'])
  })

  it('an epic sits under the vision it serves and under the design it implements', () => {
    expect(childIds(vision)).toContain('epic:plan')
    expect(childIds(design)).toContain('epic:plan')
    expect(find(design?.children ?? [], 'epic:plan')?.via).toEqual(['implements'])
  })

  it('a story sits under the epic it decomposes, at every place that epic is shown', () => {
    const underVision = find(vision?.children ?? [], 'epic:plan')
    const underDesign = find(design?.children ?? [], 'epic:plan')
    expect(childIds(underVision)).toEqual(['story:plan-pages'])
    expect(childIds(underDesign)).toEqual(['story:plan-pages'])
    expect(find(underVision?.children ?? [], 'story:plan-pages')?.via).toEqual(['decomposes'])
  })

  it('a story that also serves the vision is shown under the vision too', () => {
    expect(childIds(vision)).toContain('story:plan-pages')
    expect(find(vision?.children ?? [], 'story:plan-pages')?.via).toEqual(['serves'])
  })

  it('artifacts with no hierarchy edge and no children are Unattached', () => {
    const unattached = tree.unattached.map((node) => node.artifact.id)
    expect(unattached).toContain('review-result:shell-scope-round-1')
    expect(unattached).toContain('executable-system-specification:read-model')
    expect(unattached).not.toContain('vision:repoview')
    expect(unattached).not.toContain('story:plan-pages')
    for (const node of tree.unattached) expect(node.children).toEqual([])
  })

  it('every artifact of the store is shown somewhere', () => {
    const shown = new Set(
      outline([...tree.roots, ...tree.unattached]).map((line) => line.trim().split(' ')[0]),
    )
    for (const artifact of artifacts) expect(shown, artifact.id).toContain(artifact.id)
  })

  it('a design comes before the epics and stories that serve the same vision', () => {
    const ids = childIds(vision)
    expect(ids.indexOf('architecture-design:repoview')).toBeLessThan(ids.indexOf('epic:plan'))
  })
})

describe('buildTree edge cases (acceptance 3)', () => {
  it('only serves, designs, decomposes and implements make a child', () => {
    const tree = buildTree([
      item('vision:v'),
      item('story:a', [['depends_on', 'vision:v']]),
      item('task:t', [['implements', 'vision:v']]),
    ])
    expect(outline(tree.roots)).toEqual(['vision:v', '  task:t'])
    expect(tree.unattached.map((node) => node.artifact.id)).toEqual(['story:a'])
  })

  it('two edges to one parent show the child once, naming both', () => {
    const tree = buildTree([
      item('vision:v'),
      item('epic:e', [
        ['serves', 'vision:v'],
        ['implements', 'vision:v'],
      ]),
    ])
    expect(outline(tree.roots)).toEqual(['vision:v', '  epic:e'])
    expect(tree.roots[0]?.children[0]?.via).toEqual(['implements', 'serves'])
  })

  it('a cycle ends instead of recursing forever', () => {
    const tree = buildTree([
      item('vision:v'),
      item('epic:a', [
        ['serves', 'vision:v'],
        ['decomposes', 'epic:b'],
      ]),
      item('epic:b', [['decomposes', 'epic:a']]),
    ])
    expect(outline(tree.roots)).toEqual([
      'vision:v',
      '  epic:a',
      '    epic:b',
      '      epic:a (cycle)',
    ])
  })

  it('a chain that reaches no vision is shown under Unattached from its top', () => {
    const tree = buildTree([
      item('epic:orphan', [['serves', 'vision:missing']]),
      item('story:s', [['decomposes', 'epic:orphan']]),
    ])
    expect(tree.roots).toEqual([])
    expect(outline(tree.unattached)).toEqual(['epic:orphan', '  story:s'])
  })

  it('a cycle that reaches no vision is still shown', () => {
    const tree = buildTree([
      item('epic:a', [['decomposes', 'epic:b']]),
      item('epic:b', [['decomposes', 'epic:a']]),
    ])
    const shown = outline(tree.unattached).map((line) => line.trim())
    expect(shown).toContain('epic:a')
    expect(shown).toContain('epic:b')
  })
})

describe('filterColumns (acceptance 2)', () => {
  const columns: BoardColumn[] = [
    {
      status: 'draft',
      artifacts: [
        item('story:alpha', [], 'story'),
        { ...item('epic:beta', [], 'epic'), title: 'The Gamma epic' },
      ],
    },
    {
      status: 'zz-new',
      description: 'invented',
      artifacts: [{ ...item('story:delta', [], 'story'), tags: ['frontend'] }],
    },
  ]

  it('an empty filter keeps everything', () => {
    expect(filterColumns(columns, '', '')).toEqual(columns)
  })

  it('text matches id, title and tag, case-insensitively, and keeps every column', () => {
    const ids = (filtered: BoardColumn[]) =>
      filtered.map((column) => [column.status, column.artifacts.map((a) => a.id)])
    expect(ids(filterColumns(columns, 'ALPHA', ''))).toEqual([
      ['draft', ['story:alpha']],
      ['zz-new', []],
    ])
    expect(ids(filterColumns(columns, 'gamma', ''))).toEqual([
      ['draft', ['epic:beta']],
      ['zz-new', []],
    ])
    expect(ids(filterColumns(columns, 'front', ''))).toEqual([
      ['draft', []],
      ['zz-new', ['story:delta']],
    ])
  })

  it('kind keeps only that kind', () => {
    const filtered = filterColumns(columns, '', 'epic')
    expect(filtered.flatMap((column) => column.artifacts.map((a) => a.id))).toEqual(['epic:beta'])
  })

  it('the kinds offered are the kinds on the board, sorted', () => {
    expect(kindsOf(columns)).toEqual(['epic', 'story'])
  })
})

describe('validation (acceptance 5)', () => {
  it('no problems is valid', () => {
    expect(validation({ state: 'ready', data: { problems: [] } })).toEqual({ state: 'valid' })
  })

  it('problems are returned verbatim', () => {
    const problems = ['  a.md: sits in `story/`  ', 'b.md: second']
    expect(validation({ state: 'ready', data: { problems } })).toEqual({
      state: 'problems',
      lines: problems,
    })
  })

  it('a failed validate that printed its problems shows them', () => {
    const result = failedWith(502, {
      tool: 'aep',
      exit: 1,
      stderr: 'warning: 1 planning document could not be read\n',
      stdout: JSON.stringify({ problems: ['story/x.md: declares kind storyy'] }),
    })
    expect(validation(result)).toEqual({
      state: 'problems',
      lines: ['story/x.md: declares kind storyy'],
    })
  })

  it('a failed validate without problems shows its stderr', () => {
    const result = failedWith(502, {
      tool: 'aep',
      exit: 2,
      stderr: 'error: no store here\n',
      stdout: '',
    })
    expect(validation(result)).toEqual({ state: 'error', text: 'error: no store here\n' })
  })

  it('any other failure shows the client message', () => {
    expect(validation({ state: 'error', status: 500, message: 'HTTP 500' })).toEqual({
      state: 'error',
      text: 'HTTP 500',
    })
  })
})

describe('toolFailure', () => {
  it('reads the 502 body', () => {
    const body = { tool: 'aep', exit: 3, stderr: 'boom' }
    const result = failedWith(502, body)
    expect(toolFailure(result)).toEqual(body)
  })

  it('is null for a result without a tool body', () => {
    expect(toolFailure({ state: 'error', status: 502, message: 'HTTP 502' })).toBeNull()
    expect(toolFailure({ state: 'ready', data: {} })).toBeNull()
    expect(toolFailure(failedWith(400, { error: 'id' }))).toBeNull()
  })
})

describe('relations into an artifact (acceptance 4)', () => {
  it('lists every artifact whose relation targets the id', () => {
    expect(incomingRelations(artifacts, 'epic:plan')).toEqual([
      { relation: 'depends_on', source: 'epic:static-export' },
      { relation: 'decomposes', source: 'story:plan-pages' },
    ])
  })
})

describe('history order (acceptance 4)', () => {
  it('is oldest first whatever order aep printed', () => {
    const history = historyFixture as HistoryEntry[]
    const reversed = [...history].reverse()
    expect(oldestFirst(reversed).map((entry) => entry.revision)).toEqual([15, 16])
  })
})

describe('the plan API paths', () => {
  const fetchMock = vi.fn<typeof fetch>()

  beforeEach(() => {
    fetchMock.mockReset()
    fetchMock.mockResolvedValue(new Response('{}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('a document of the wrong shape is an error naming the route, never a crash', async () => {
    const wrong = [
      [() => loadBoard(), [{ status: 'draft' }], 'plan/board'],
      [() => loadBoard(), { sources: [] }, 'plan/board'],
      [() => loadArtifacts(), { sources: [] }, 'plan/artifacts'],
      [() => loadArtifacts(), [{ id: 'story:a', kind: 'story' }], 'plan/artifacts'],
      [() => loadValidate(), [], 'plan/validate'],
      [() => loadArtifact('story:a'), { sources: [] }, 'plan/artifacts/story:a'],
      [() => loadHistory('story:a'), {}, 'plan/artifacts/story:a/history'],
      [() => loadExplain('story:a'), [], 'plan/artifacts/story:a/explain'],
    ] as const
    for (const [load, document, path] of wrong) {
      fetchMock.mockResolvedValueOnce(new Response(JSON.stringify(document), { status: 200 }))
      const result = await load()
      expect(result.state, path).toBe('error')
      if (result.state === 'error') expect(result.message, path).toContain(path)
    }
  })

  it('blocked_by is the list of { blocker, type } aep prints', async () => {
    const strings = [{ ...blockedItem, blocked_by: ['dependency-blocker:x'] }]
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify(strings), { status: 200 }))
    expect((await loadArtifacts()).state).toBe('error')
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify([blockedItem]), { status: 200 }))
    expect(await loadArtifacts()).toEqual({ state: 'ready', data: [blockedItem] })
    const explain = { ...explainBlocked, blocked_by: ['dependency-blocker:x'] }
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify(explain), { status: 200 }))
    expect((await loadExplain('story:a')).state).toBe('error')
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify(explainBlocked), { status: 200 }))
    expect(await loadExplain('story:a')).toEqual({ state: 'ready', data: explainBlocked })
  })

  it('a well-formed document passes through', async () => {
    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify(artifactsFixture), { status: 200 }))
    const result = await loadArtifacts()
    expect(result).toEqual({ state: 'ready', data: artifactsFixture })
  })

  it('each loader reads its /api/plan route', async () => {
    await loadBoard()
    await loadArtifacts()
    await loadValidate()
    await loadArtifact('story:plan-pages')
    await loadHistory('story:plan-pages')
    await loadExplain('story:plan-pages')
    expect(fetchMock.mock.calls.map((call) => call[0] as string)).toEqual([
      '/api/plan/board',
      '/api/plan/artifacts',
      '/api/plan/validate',
      '/api/plan/artifacts/story%3Aplan-pages',
      '/api/plan/artifacts/story%3Aplan-pages/history',
      '/api/plan/artifacts/story%3Aplan-pages/explain',
    ])
  })
})

// Captured read-only: codegate's epic:offline-dependency-evaluation (explain and history) and a
// review outcome from aep's own store. Expected lines follow `aep plan artifact explain|history`
// text output, with `->` written as `→`.
describe('explain and history lines carry what aep prints (acceptance 4)', () => {
  const explain = explainExecuted as Explain
  const history = historyExecuted as HistoryEntry[]

  it('a step names its revision, executor and correlation', () => {
    expect(explain.reached?.map(stepDetails)).toEqual([
      'revision 2',
      'revision 3',
      'revision 4, executed by agent:codegate-wave001, correlation codegate-wave001',
    ])
  })

  it('a step names its actor when aep prints one', () => {
    expect(stepDetails({ from: 'a', to: 'b', revision: 1, actor: 'human:x' })).toBe(
      'revision 1, by human:x',
    )
  })

  it('an evidence record names kind, source, reference, when observed and admitted', () => {
    expect(evidenceText(explain.reached?.[2]?.rested_on?.[0])).toBe(
      'test_result from Single child story implemented after integration task check exit0 (verification-report:codegate-wave1), observed 2026-10-02T12:55:17Z, admitted at revision 3',
    )
  })

  it('a history move names its executor and correlation; evidence names what was recorded', () => {
    expect(history.map(historyText)).toEqual([
      'moved draft → proposed',
      'moved proposed → active',
      'test_result recorded from Single child story implemented after integration task check exit0 (verification-report:codegate-wave1)',
      'moved active → implemented, executed by agent:codegate-wave001, correlation codegate-wave001',
    ])
  })

  it('a review outcome names the review and its outcome', () => {
    expect((historyReview as HistoryEntry[]).map(historyText)).toEqual([
      'review_outcome recorded from review-result:adversary-stage-dir-pass-1: review-result:adversary-stage-dir-pass-1 was no-op',
    ])
  })

  it('a change of a kind it does not know keeps every field', () => {
    const entry = { at: 'x', revision: 1, change: { change: 'renamed', from: 'a:b', note: 'n' } }
    expect(historyText(entry)).toBe('renamed {"from":"a:b","note":"n"}')
  })
})
