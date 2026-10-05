import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import boardFixture from '../__fixtures__/plan/board.json'
import { failedWith } from '../__fixtures__/plan/failures'
import { apiGet, type ApiResult } from '../api/client'
import type { BoardColumn } from '../api/plan'
import { createAppRouter } from '../router'
import PlanBoardPage from './PlanBoardPage.vue'

// story:plan-pages acceptances 2 and 5.

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  apiGet: vi.fn(),
}))

const board = boardFixture as BoardColumn[]

const VALID = { store: 'store', files_read: 2, artifacts: 2, problems: [] }

function answer(answers: Record<string, ApiResult<unknown>>) {
  vi.mocked(apiGet).mockImplementation((path: string) =>
    Promise.resolve(
      (answers[path] ?? { state: 'error', status: 404, message: 'HTTP 404' }) as ApiResult<never>,
    ),
  )
}

async function mountBoard(
  columns: unknown,
  validate: ApiResult<unknown> = { state: 'ready', data: VALID },
): Promise<VueWrapper> {
  answer({ 'plan/board': { state: 'ready', data: columns }, 'plan/validate': validate })
  const router = createAppRouter(createMemoryHistory())
  await router.push('/plan')
  await router.isReady()
  const wrapper = mount(PlanBoardPage, { global: { plugins: [router] } })
  await flushPromises()
  return wrapper
}

function columnStatuses(wrapper: VueWrapper): string[] {
  return wrapper.findAll('[data-test="board-column"]').map((c) => c.attributes('data-status') ?? '')
}

function cardIds(wrapper: VueWrapper): string[] {
  return wrapper.findAll('[data-test="plan-card"]').map((c) => c.attributes('data-id') ?? '')
}

beforeEach(() => {
  vi.mocked(apiGet).mockReset()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Board columns come from aep (acceptance 2)', () => {
  it('reads plan/board and plan/validate', async () => {
    await mountBoard(board)
    expect(
      vi
        .mocked(apiGet)
        .mock.calls.map((call) => call[0])
        .sort(),
    ).toEqual(['plan/board', 'plan/validate'])
  })

  it('one column per board entry, in its order, named by its status', async () => {
    const wrapper = await mountBoard(board)
    expect(columnStatuses(wrapper)).toEqual(board.map((column) => column.status))
    const headings = wrapper.findAll('[data-test="board-column"] [data-test="column-status"]')
    expect(headings.map((h) => h.text())).toEqual(board.map((column) => column.status))
  })

  it('an invented status and its description show as a column', async () => {
    const columns = [
      ...board,
      {
        status: 'zz-new',
        description: 'A status no list in the web code knows.',
        artifacts: [
          {
            id: 'story:zz',
            kind: 'story',
            status: 'zz-new',
            title: 'Fresh',
            relations: [],
          },
        ],
      },
    ]
    const wrapper = await mountBoard(columns)
    expect(columnStatuses(wrapper).at(-1)).toBe('zz-new')
    const column = wrapper.get('[data-test="board-column"][data-status="zz-new"]')
    expect(column.get('[data-test="column-status"]').text()).toBe('zz-new')
    expect(column.get('[data-test="column-description"]').text()).toBe(
      'A status no list in the web code knows.',
    )
    expect(column.findAll('[data-test="plan-card"]').map((c) => c.attributes('data-id'))).toEqual([
      'story:zz',
    ])
  })

  it('a column without a description shows none, never an invented one', async () => {
    const wrapper = await mountBoard([{ status: 'draft', artifacts: [] }])
    expect(wrapper.find('[data-test="column-description"]').exists()).toBe(false)
  })

  it('each artifact is a card with kind, id and title linking to its page', async () => {
    const wrapper = await mountBoard(board)
    const card = wrapper.get('[data-test="plan-card"][data-id="story:plan-pages"]')
    expect(card.get('[data-test="card-kind"]').text()).toBe('story')
    expect(card.get('[data-test="card-id"]').text()).toBe('story:plan-pages')
    expect(card.get('[data-test="card-title"]').text()).toBe('Plan board, tree and artifact pages')
    expect(card.get('a').attributes('href')).toBe('/plan/artifact/story:plan-pages')
    expect(cardIds(wrapper)).toHaveLength(board.flatMap((column) => column.artifacts).length)
  })

  it('a column shows how many cards it holds', async () => {
    const wrapper = await mountBoard(board)
    const first = board[0]
    if (first === undefined) throw new Error('empty board fixture')
    const column = wrapper.get(`[data-test="board-column"][data-status="${first.status}"]`)
    expect(column.get('[data-test="column-count"]').text()).toBe(String(first.artifacts.length))
  })
})

describe('Board filters (acceptance 2)', () => {
  it('the filter box matches title and id', async () => {
    const wrapper = await mountBoard(board)
    await wrapper.get('[data-test="board-filter"]').setValue('plan BOARD, tree')
    expect(cardIds(wrapper)).toEqual(['story:plan-pages'])
    await wrapper.get('[data-test="board-filter"]').setValue('STORY:PAGE-FRAME')
    expect(cardIds(wrapper)).toEqual(['story:page-frame'])
  })

  it('the filter box matches a tag', async () => {
    const wrapper = await mountBoard([
      {
        status: 'draft',
        artifacts: [
          { id: 'story:a', kind: 'story', status: 'draft', title: 'A', relations: [], tags: [] },
          {
            id: 'story:b',
            kind: 'story',
            status: 'draft',
            title: 'B',
            relations: [],
            tags: ['frontend'],
          },
        ],
      },
    ])
    await wrapper.get('[data-test="board-filter"]').setValue('frontend')
    expect(cardIds(wrapper)).toEqual(['story:b'])
  })

  it('the kind filter offers the kinds on the board and keeps one', async () => {
    const wrapper = await mountBoard(board)
    const options = wrapper.findAll('[data-test="kind-filter"] option').map((o) => o.text())
    expect(options).toEqual([
      'all kinds',
      ...[...new Set(board.flatMap((c) => c.artifacts.map((a) => a.kind)))].sort(),
    ])
    await wrapper.get('[data-test="kind-filter"]').setValue('epic')
    expect(cardIds(wrapper).every((id) => id.startsWith('epic:'))).toBe(true)
    expect(cardIds(wrapper)).toContain('epic:plan')
  })

  it('columns stay when the filter empties them', async () => {
    const wrapper = await mountBoard(board)
    await wrapper.get('[data-test="board-filter"]').setValue('no artifact is called this')
    expect(cardIds(wrapper)).toEqual([])
    expect(columnStatuses(wrapper)).toEqual(board.map((column) => column.status))
  })
})

describe('Board header shows validate (acceptance 5)', () => {
  it('"valid" when validate reports no problems', async () => {
    const wrapper = await mountBoard(board)
    expect(wrapper.get('[data-test="validate"]').text()).toBe('valid')
  })

  it('the defect lines verbatim when validate reports problems', async () => {
    const lines = ['story/a.md: sits in `story/`, but declares `kind: storyy`', 'b.md: second']
    const wrapper = await mountBoard(
      board,
      failedWith(502, {
        tool: 'aep',
        exit: 1,
        stderr: '',
        stdout: JSON.stringify({ problems: lines }),
      }),
    )
    const shown = wrapper.findAll('[data-test="validate-problem"]').map((li) => li.text())
    expect(shown).toEqual(lines)
    expect(wrapper.get('[data-test="validate"]').text()).not.toContain('valid\n')
  })

  it('a validate that could not run shows why', async () => {
    const wrapper = await mountBoard(
      board,
      failedWith(503, { tool: 'aep', exit: null, stderr: 'aep not found on PATH' }),
    )
    expect(wrapper.get('[data-test="validate"]').text()).toContain('aep not found on PATH')
  })
})

describe('Board failures', () => {
  it('a failed board shows the stderr aep printed', async () => {
    answer({
      'plan/board': failedWith(502, {
        tool: 'aep',
        exit: 2,
        stderr: 'error: the store is unreadable',
      }),
      'plan/validate': { state: 'ready', data: VALID },
    })
    const router = createAppRouter(createMemoryHistory())
    await router.push('/plan')
    const wrapper = mount(PlanBoardPage, { global: { plugins: [router] } })
    await flushPromises()
    expect(wrapper.get('[data-test="tool-failure"]').text()).toContain(
      'error: the store is unreadable',
    )
    expect(wrapper.find('[data-test="board-column"]').exists()).toBe(false)
  })
})
