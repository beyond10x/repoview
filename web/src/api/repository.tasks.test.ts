import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { serveRoutes } from '../__fixtures__/repository/fetch'
import { TOKEN_REJECTED_MESSAGE } from './snapshot'
import { loadTasks, normaliseTasks, type Tasks } from './repository'

// story:taskfile-tasks acceptance 1 (the `/api/tasks` wire shape) as the SPA reads it.

const tasks: Tasks = {
  file: 'Taskfile.yml',
  tasks: [
    {
      name: 'check',
      desc: 'The gate',
      summary: 'Runs every check.\n',
      internal: false,
      aliases: ['c'],
    },
    { name: 'docs:build', desc: null, summary: null, internal: true, aliases: [] },
  ],
  refused: [{ include: 'remote', taskfile: 'https://example.invalid/T.yml', reason: 'remote' }],
  truncated: false,
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('normaliseTasks', () => {
  it('a well-formed payload is unchanged', () => {
    expect(normaliseTasks(JSON.parse(JSON.stringify(tasks)))).toEqual(tasks)
  })

  it('a payload missing everything is an empty list', () => {
    expect(normaliseTasks(null)).toEqual({ file: '', tasks: [], refused: [], truncated: false })
    expect(normaliseTasks([{ name: 'x' }])).toEqual({
      file: '',
      tasks: [],
      refused: [],
      truncated: false,
    })
  })

  it('drops nameless tasks, keeps only string aliases and true internal flags', () => {
    expect(
      normaliseTasks({
        file: 'Taskfile.yml',
        tasks: [
          { desc: 'nameless' },
          { name: 'a', desc: 3, summary: '', internal: 'yes', aliases: ['x', 1, null] },
        ],
        refused: [{ include: 'i', taskfile: null, reason: 'no `taskfile` path' }],
        truncated: true,
      }),
    ).toEqual({
      file: 'Taskfile.yml',
      tasks: [{ name: 'a', desc: null, summary: null, internal: false, aliases: ['x'] }],
      refused: [{ include: 'i', taskfile: null, reason: 'no `taskfile` path' }],
      truncated: true,
    })
  })
})

describe('loadTasks', () => {
  it('reads /api/tasks', async () => {
    serveRoutes({ '/api/tasks': tasks })
    expect(await loadTasks()).toEqual({ state: 'ready', data: tasks })
  })

  it('no Taskfile (404) is absent', async () => {
    serveRoutes({})
    expect(await loadTasks()).toEqual({
      state: 'unavailable',
      reason: 'absent',
      message: 'Taskfile.yml absent',
    })
  })

  it('an unreadable Taskfile (503) names the diagnostic', async () => {
    serveRoutes({
      '/api/tasks': {
        status: 503,
        body: { availability: 'Failed', diagnostic: 'Taskfile.yml: invalid YAML: at line 2' },
      },
    })
    expect(await loadTasks()).toEqual({
      state: 'unavailable',
      reason: 'error',
      message: 'Taskfile.yml: invalid YAML: at line 2',
    })
  })

  it('a 503 without a diagnostic still says the Taskfile could not be read', async () => {
    serveRoutes({ '/api/tasks': { status: 503 } })
    expect(await loadTasks()).toEqual({
      state: 'unavailable',
      reason: 'error',
      message: 'the Taskfile could not be read',
    })
  })

  it('a rejected token says so', async () => {
    serveRoutes({ '/api/tasks': { status: 403 } })
    expect(await loadTasks()).toEqual({
      state: 'unavailable',
      reason: 'error',
      message: TOKEN_REJECTED_MESSAGE,
    })
  })
})
