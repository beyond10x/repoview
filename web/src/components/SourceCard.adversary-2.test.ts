import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { Source } from '../api/snapshot'
import SourceCard from './SourceCard.vue'

// A repository with no commit yet (`git init`, before the first commit): `git rev-parse --verify
// --quiet HEAD` exits 1, so the server sends `summary.head` = null (crates/repoview-sources/src/vcs.rs,
// `Outcome::Failure { .. } => None`), while `git branch --show-current` still prints the branch.
// The card's 'no commit' fallback is wired to `head === ""`, which the server never sends.
const unbornVcs: Source = {
  source_id: 'vcs',
  kind: 'Vcs',
  location: '.',
  availability: 'Present',
  tool: 'git',
  tool_path: '/usr/bin/git',
  tool_version: 'git version 2.51.0',
  diagnostic: null,
  summary: { branch: 'main', head: null, dirty: 1 },
}

describe('adversary pass 2: vcs card on a repository with no commit', () => {
  it('summary.head null (unborn branch) renders the head row as "no commit", not nothing', () => {
    const wrapper = mount(SourceCard, { props: { source: unbornVcs } })
    const dts = wrapper.findAll('dt').map((dt) => dt.text())
    const dds = wrapper.findAll('dd').map((dd) => dd.text())
    expect(dts, 'vcs summary labels').toContain('head')
    expect(dds[dts.indexOf('head')]).toBe('no commit')
  })
})
