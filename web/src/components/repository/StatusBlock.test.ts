import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import {
  HEAD,
  PARENT,
  vcsDetached,
  vcsEmpty,
  vcsFull,
  vcsGoneUpstream,
} from '../../__fixtures__/repository'
import StatusBlock from './StatusBlock.vue'

// story:repository-page acceptance 2 and 4: the status block.

describe('StatusBlock', () => {
  it('shows branch, short head, upstream with ahead and behind', () => {
    const wrapper = mount(StatusBlock, { props: { vcs: vcsFull } })
    expect(wrapper.get('[data-test="branch"]').text()).toBe('main')
    const head = wrapper.get('[data-test="head"]')
    expect(head.text()).toBe(HEAD.slice(0, 8))
    expect(head.attributes('title')).toBe(HEAD)
    const upstream = wrapper.get('[data-test="upstream"]').text()
    expect(upstream).toContain('origin/main')
    expect(wrapper.get('[data-test="ahead"]').text()).toBe('↑2')
    expect(wrapper.get('[data-test="behind"]').text()).toBe('↓1')
  })

  it('lists every dirty file with its status letters, a rename with its old path', () => {
    const wrapper = mount(StatusBlock, { props: { vcs: vcsFull } })
    const rows = wrapper.findAll('[data-test="dirty-file"]')
    expect(rows.map((row) => row.get('[data-test="dirty-status"]').text())).toEqual([
      '.M',
      'A.',
      'R.',
      'UU',
      '??',
    ])
    expect(rows.map((row) => row.get('[data-test="dirty-path"]').text())).toEqual([
      'src/lib.rs',
      'src/new.rs',
      'docs/old-guide.md → docs/guide.md',
      'conflict.txt',
      'scratch.txt',
    ])
    expect(wrapper.get('[data-test="dirty-count"]').text()).toBe('5 changed')
  })

  it('a detached HEAD says so and shows no upstream', () => {
    const wrapper = mount(StatusBlock, { props: { vcs: vcsDetached } })
    expect(wrapper.get('[data-test="branch"]').text()).toBe('detached HEAD')
    expect(wrapper.get('[data-test="head"]').text()).toBe(PARENT.slice(0, 8))
    expect(wrapper.get('[data-test="upstream"]').text()).toBe('no upstream')
    expect(wrapper.find('[data-test="ahead"]').exists()).toBe(false)
    expect(wrapper.get('[data-test="dirty-count"]').text()).toBe('clean')
    expect(wrapper.findAll('[data-test="dirty-file"]')).toHaveLength(0)
  })

  it('an empty repository names its unborn branch and has no head', () => {
    const wrapper = mount(StatusBlock, { props: { vcs: vcsEmpty } })
    expect(wrapper.get('[data-test="branch"]').text()).toBe('main')
    expect(wrapper.get('[data-test="head"]').text()).toBe('no commits yet')
    expect(wrapper.get('[data-test="upstream"]').text()).toBe('no upstream')
    expect(wrapper.get('[data-test="dirty-count"]').text()).toBe('clean')
  })

  it('an upstream without counts is named, with the counts unknown', () => {
    const wrapper = mount(StatusBlock, { props: { vcs: vcsGoneUpstream } })
    expect(wrapper.get('[data-test="upstream"]').text()).toContain('origin/main')
    expect(wrapper.get('[data-test="upstream"]').text()).toContain('ahead/behind unknown')
    expect(wrapper.find('[data-test="ahead"]').exists()).toBe(false)
  })
})
