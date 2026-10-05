import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  documentBodies,
  documents,
  noDocuments,
  vcsDetached,
  vcsEmpty,
  vcsFull,
  vcsWorktreesFailed,
} from '../__fixtures__/repository'
import { requestedUrls, serveRoutes, type Routes } from '../__fixtures__/repository/fetch'
import { allPresent } from '../__fixtures__/snapshots'
import { TOKEN_REJECTED_MESSAGE } from '../api/snapshot'
import RepositoryPage from './RepositoryPage.vue'

// story:repository-page acceptance 2 and 4: the whole page against stubbed API routes.

const FULL: Routes = {
  '/api/vcs': vcsFull,
  '/api/docs': documents,
  '/api/docs/README.md': { name: 'README.md', markdown: documentBodies['README.md'] },
}

async function mountPage(routes: Routes): Promise<VueWrapper> {
  serveRoutes(routes)
  const wrapper = mount(RepositoryPage)
  await flushPromises()
  return wrapper
}

function unavailable(wrapper: VueWrapper, block: string) {
  return wrapper.get(`[data-test="unavailable"][data-block="${block}"]`)
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('RepositoryPage', () => {
  it('shows status, commits, tags, remotes, worktrees, documents and the task block', async () => {
    const wrapper = await mountPage(FULL)
    expect(wrapper.get('h1').text()).toBe('Repository')
    expect(wrapper.get('[data-test="branch"]').text()).toBe('main')
    expect(wrapper.findAll('[data-test="dirty-file"]')).toHaveLength(5)
    expect(wrapper.findAll('[data-test="commit"]')).toHaveLength(2)
    expect(wrapper.findAll('[data-test="tag"]')).toHaveLength(2)
    expect(wrapper.findAll('[data-test="remote"]')).toHaveLength(1)
    expect(wrapper.findAll('[data-test="worktree"]')).toHaveLength(3)
    expect(wrapper.findAll('[data-test="doc-tab"]').map((tab) => tab.text())).toEqual([
      'README.md',
      'AGENTS.md',
      'CHANGELOG.md',
    ])
    expect(wrapper.get('[data-test="doc-absent"]').text()).toBe('STATUS.md absent')
    expect(wrapper.get('[data-test="doc-panel"] h1').text()).toBe('Example')
    // story:taskfile-tasks: the task block is there; FULL serves no Taskfile, so it reads absent.
    expect(wrapper.get('[data-block="tasks"] h2').text()).toBe('Tasks')
    expect(wrapper.get('[data-test="unavailable"][data-block="tasks"]').text()).toBe(
      'Taskfile.yml absent',
    )
  })

  it('reads its four routes once each', async () => {
    const mock = serveRoutes(FULL)
    mount(RepositoryPage)
    await flushPromises()
    expect(requestedUrls(mock).sort()).toEqual([
      '/api/docs',
      '/api/docs/README.md',
      '/api/tasks',
      '/api/vcs',
    ])
  })

  it('a detached HEAD reads "detached HEAD"', async () => {
    const wrapper = await mountPage({ ...FULL, '/api/vcs': vcsDetached })
    expect(wrapper.get('[data-test="branch"]').text()).toBe('detached HEAD')
    expect(wrapper.get('[data-test="upstream"]').text()).toBe('no upstream')
  })

  it('an empty repository: unborn branch, no commits, tags or remotes, no documents', async () => {
    const wrapper = await mountPage({
      '/api/vcs': vcsEmpty,
      '/api/docs': noDocuments,
    })
    expect(wrapper.get('[data-test="branch"]').text()).toBe('main')
    expect(wrapper.get('[data-test="head"]').text()).toBe('no commits yet')
    expect(wrapper.get('[data-block="commits"] [data-test="empty"]').text()).toBe('no commits yet')
    expect(wrapper.get('[data-block="tags"] [data-test="empty"]').text()).toBe('no tags')
    expect(wrapper.get('[data-block="remotes"] [data-test="empty"]').text()).toBe('no remotes')
    expect(wrapper.findAll('[data-test="doc-absent"]')).toHaveLength(4)
  })

  it('a worktree listing Git could not produce shows its error; the other blocks still show', async () => {
    const wrapper = await mountPage({ ...FULL, '/api/vcs': vcsWorktreesFailed })
    expect(wrapper.get('[data-block="worktrees"] [data-test="block-error"]').text()).toBe(
      "error: unknown switch `z'",
    )
    expect(wrapper.findAll('[data-test="worktree"]')).toHaveLength(0)
    expect(wrapper.find('[data-block="tags"] [data-test="block-error"]').exists()).toBe(false)
    expect(wrapper.findAll('[data-test="commit"]')).toHaveLength(2)
    expect(wrapper.findAll('[data-test="tag"]')).toHaveLength(2)
  })

  it('outside Git the Git blocks say so and the documents still show', async () => {
    const wrapper = await mountPage({
      ...FULL,
      '/api/vcs': { status: 404, body: { availability: 'Absent', tool: 'git' } },
    })
    expect(unavailable(wrapper, 'vcs').text()).toBe('not a Git work tree')
    expect(unavailable(wrapper, 'vcs').attributes('data-reason')).toBe('absent')
    expect(wrapper.find('[data-test="commit"]').exists()).toBe(false)
    expect(wrapper.findAll('[data-test="doc-tab"]')).toHaveLength(3)
  })

  it('git missing or failing is a 503 the page names', async () => {
    const wrapper = await mountPage({
      ...FULL,
      '/api/vcs': { status: 503, body: { availability: 'ToolMissing', tool: 'git' } },
    })
    expect(unavailable(wrapper, 'vcs').text()).toBe(
      'git could not be run here (not on PATH, or it failed)',
    )
    expect(unavailable(wrapper, 'vcs').attributes('data-reason')).toBe('error')
  })

  it('a rejected token is shown on every block', async () => {
    const forbidden = { status: 403 }
    const wrapper = await mountPage({
      '/api/vcs': forbidden,
      '/api/docs': forbidden,
    })
    for (const block of ['vcs', 'docs']) {
      expect(unavailable(wrapper, block).text()).toBe(TOKEN_REJECTED_MESSAGE)
    }
  })

  it('a payload of the wrong shape renders empty blocks instead of breaking the page', async () => {
    const wrapper = await mountPage({
      '/api/vcs': allPresent,
      '/api/docs': allPresent,
    })
    expect(wrapper.get('h1').text()).toBe('Repository')
    expect(wrapper.get('[data-test="branch"]').text()).toBe('detached HEAD')
    expect(wrapper.get('[data-block="commits"] [data-test="empty"]').text()).toBe('no commits yet')
  })
})
