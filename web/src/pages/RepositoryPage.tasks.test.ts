import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { documentBodies, documents, vcsFull } from '../__fixtures__/repository'
import { requestedUrls, serveRoutes, type Routes } from '../__fixtures__/repository/fetch'
import RepositoryPage from './RepositoryPage.vue'

// story:taskfile-tasks acceptance 2: the Repository page shows the task list, or "absent".

const BASE: Routes = {
  '/api/vcs': vcsFull,
  '/api/docs': documents,
  '/api/docs/README.md': { name: 'README.md', markdown: documentBodies['README.md'] },
}

const TASKS = {
  file: 'Taskfile.yml',
  tasks: [
    { name: 'check', desc: 'The gate', summary: null, internal: false, aliases: [] },
    { name: 'build', desc: 'Build it', summary: null, internal: false, aliases: [] },
  ],
  refused: [],
  truncated: false,
}

async function mountPage(routes: Routes): Promise<VueWrapper> {
  serveRoutes(routes)
  const wrapper = mount(RepositoryPage)
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('RepositoryPage tasks', () => {
  it('shows the Taskfile tasks in a Tasks block', async () => {
    const wrapper = await mountPage({ ...BASE, '/api/tasks': TASKS })
    const block = wrapper.get('[data-block="tasks"]')
    expect(block.get('h2').text()).toBe('Tasks')
    expect(block.findAll('[data-test="task-name"]').map((name) => name.text())).toEqual([
      'check',
      'build',
    ])
  })

  it('without a Taskfile the block says absent and the rest of the page still shows', async () => {
    const wrapper = await mountPage(BASE)
    const absent = wrapper.get('[data-test="unavailable"][data-block="tasks"]')
    expect(absent.text()).toBe('Taskfile.yml absent')
    expect(absent.attributes('data-reason')).toBe('absent')
    expect(wrapper.findAll('[data-test="commit"]')).toHaveLength(2)
  })

  it('an unreadable Taskfile shows its diagnostic', async () => {
    const wrapper = await mountPage({
      ...BASE,
      '/api/tasks': {
        status: 503,
        body: { availability: 'Failed', diagnostic: 'Taskfile.yml: larger than 1 MiB; not read' },
      },
    })
    const failed = wrapper.get('[data-test="unavailable"][data-block="tasks"]')
    expect(failed.text()).toBe('Taskfile.yml: larger than 1 MiB; not read')
    expect(failed.attributes('data-reason')).toBe('error')
  })

  it('reads /api/tasks once', async () => {
    const mock = serveRoutes({ ...BASE, '/api/tasks': TASKS })
    mount(RepositoryPage)
    await flushPromises()
    expect(requestedUrls(mock).filter((url) => url === '/api/tasks')).toHaveLength(1)
  })
})
