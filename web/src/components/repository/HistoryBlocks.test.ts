import { mount, type VueWrapper } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { HEAD, PARENT, vcsDetached, vcsEmpty, vcsFull } from '../../__fixtures__/repository'
import CommitsTable from './CommitsTable.vue'
import RemotesList from './RemotesList.vue'
import TagsTable from './TagsTable.vue'
import WorktreesTable from './WorktreesTable.vue'

// story:repository-page acceptance 2 and 4: commits, tags, remotes and worktrees.

describe('CommitsTable', () => {
  it('lists each commit: short sha, subject, author, date', () => {
    const wrapper = mount(CommitsTable, { props: { commits: vcsFull.commits } })
    const rows = wrapper.findAll('[data-test="commit"]')
    expect(rows).toHaveLength(2)
    const first = rows[0]
    if (first === undefined) throw new Error('no first row')
    expect(first.get('[data-test="commit-sha"]').text()).toBe(HEAD.slice(0, 8))
    expect(first.get('[data-test="commit-sha"]').attributes('title')).toBe(HEAD)
    expect(first.get('[data-test="commit-subject"]').text()).toBe('feat: the repository page')
    expect(first.get('[data-test="commit-author"]').text()).toBe('Ada Lovelace')
    expect(first.get('[data-test="commit-date"]').text()).toBe('2026-10-05 14:03')
    expect(first.get('time').attributes('datetime')).toBe('2026-10-05T14:03:11+02:00')
  })

  it('renders a subject as text, never as markup', () => {
    const wrapper = mount(CommitsTable, { props: { commits: vcsFull.commits } })
    expect(wrapper.find('b').exists()).toBe(false)
    expect(wrapper.findAll('[data-test="commit-subject"]')[1]?.text()).toBe('fix: <b>not bold</b>')
  })

  it('an empty repository has no commits yet', () => {
    const wrapper = mount(CommitsTable, { props: { commits: vcsEmpty.commits } })
    expect(wrapper.findAll('[data-test="commit"]')).toHaveLength(0)
    expect(wrapper.get('[data-test="empty"]').text()).toBe('no commits yet')
  })
})

describe('TagsTable', () => {
  it('lists tags newest first with the commit they point at', () => {
    const wrapper = mount(TagsTable, { props: { tags: vcsFull.tags } })
    const rows = wrapper.findAll('[data-test="tag"]')
    expect(rows.map((row) => row.get('[data-test="tag-name"]').text())).toEqual(['0.2.0', '0.1.0'])
    expect(rows[1]?.get('[data-test="tag-sha"]').text()).toBe(PARENT.slice(0, 8))
    expect(rows[1]?.get('[data-test="tag-date"]').text()).toBe('2026-10-01 10:00')
  })

  it('no tags says so', () => {
    const wrapper = mount(TagsTable, { props: { tags: [] } })
    expect(wrapper.get('[data-test="empty"]').text()).toBe('no tags')
  })
})

describe('RemotesList', () => {
  it('lists each remote with its URL', () => {
    const wrapper = mount(RemotesList, { props: { remotes: vcsFull.remotes } })
    const rows = wrapper.findAll('[data-test="remote"]')
    expect(rows).toHaveLength(1)
    expect(rows[0]?.text()).toContain('origin')
    expect(rows[0]?.text()).toContain('https://github.com/o/r.git')
  })

  it('no remotes says so', () => {
    const wrapper = mount(RemotesList, { props: { remotes: [] } })
    expect(wrapper.get('[data-test="empty"]').text()).toBe('no remotes')
  })
})

describe('WorktreesTable', () => {
  it('lists each worktree: path, branch or detached, short head, locked and prunable', () => {
    const wrapper = mount(WorktreesTable, { props: { worktrees: vcsFull.worktrees } })
    const rows = wrapper.findAll('[data-test="worktree"]')
    expect(rows.map((row) => row.get('[data-test="worktree-path"]').text())).toEqual([
      '/work/example',
      '/work/example-feature',
      '/work/example-gone',
    ])
    expect(rows.map((row) => row.get('[data-test="worktree-branch"]').text())).toEqual([
      'main',
      'feature',
      'detached',
    ])
    expect(rows[0]?.get('[data-test="worktree-head"]').text()).toBe(HEAD.slice(0, 8))
    expect(rows.map((row) => row.get('[data-test="worktree-flags"]').text())).toEqual([
      '',
      'locked',
      'prunable',
    ])
  })

  it('a detached main worktree reads detached', () => {
    const wrapper = mount(WorktreesTable, { props: { worktrees: vcsDetached.worktrees } })
    expect(wrapper.get('[data-test="worktree-branch"]').text()).toBe('detached')
  })

  it('an unborn worktree has no commits yet', () => {
    const wrapper = mount(WorktreesTable, { props: { worktrees: vcsEmpty.worktrees } })
    expect(wrapper.get('[data-test="worktree-head"]').text()).toBe('no commits yet')
  })
})

// Correction round 2: a block Git could not read shows Git's error, not an empty list.
describe('a block with an error', () => {
  const blocks: Array<[string, (error: string | null) => VueWrapper]> = [
    ['TagsTable', (error) => mount(TagsTable, { props: { tags: [], error } })],
    ['RemotesList', (error) => mount(RemotesList, { props: { remotes: [], error } })],
    ['WorktreesTable', (error) => mount(WorktreesTable, { props: { worktrees: [], error } })],
  ]
  for (const [name, mountWith] of blocks) {
    it(`${name} shows the error instead of an empty list`, () => {
      const wrapper = mountWith('fatal: broken')
      expect(wrapper.get('[data-test="block-error"]').text()).toBe('fatal: broken')
      expect(wrapper.find('[data-test="empty"]').exists()).toBe(false)
    })

    it(`${name} without an error shows no error`, () => {
      const wrapper = mountWith(null)
      expect(wrapper.find('[data-test="block-error"]').exists()).toBe(false)
    })
  }
})
