import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { VCS_HEAD } from '../__fixtures__/snapshots'
import type { Source } from '../api/snapshot'
import SourceCard from './SourceCard.vue'

// On a detached HEAD `git branch --show-current` prints nothing, and the server puts
// `summary.branch` = "" (crates/repoview-sources/src/vcs.rs: `branch.trim()`). A CI checkout, a
// bisect or a rebase in progress all reach it.
const detachedVcs: Source = {
  source_id: 'vcs',
  kind: 'Vcs',
  location: '.',
  availability: 'Present',
  tool: 'git',
  tool_path: '/usr/bin/git',
  tool_version: 'git version 2.51.0',
  diagnostic: null,
  summary: { branch: '', head: VCS_HEAD, dirty: 0 },
}

describe('adversary: vcs card edge cases', () => {
  it('a detached HEAD (summary.branch "") renders no empty summary value', () => {
    const wrapper = mount(SourceCard, { props: { source: detachedVcs } })
    const rows = wrapper
      .findAll('dt')
      .map((dt, i) => [dt.text(), wrapper.findAll('dd')[i]?.text().trim() ?? ''])
    for (const [label, value] of rows) {
      expect(value, `vcs card "${String(label)}" value`).not.toBe('')
    }
    expect(wrapper.text()).toContain(VCS_HEAD.slice(0, 7))
  })
})
