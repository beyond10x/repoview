import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { Availability, Source, SourceKind } from '../api/snapshot'
import SourceCard from './SourceCard.vue'

const AVAILABILITIES: Availability[] = ['Present', 'Absent', 'ToolMissing', 'Failed']
const KINDS: SourceKind[] = ['Vcs', 'Planning', 'Specification', 'Quality', 'Documents']

function leafTexts(source: Source): Array<[string, string]> {
  const wrapper = mount(SourceCard, { props: { source } })
  return wrapper
    .findAll('*')
    .filter((el) => el.element.children.length === 0)
    .map((el) => [
      el.element.tagName.toLowerCase() + '.' + el.classes().join('.'),
      el.text().trim(),
    ])
}

describe('no server string renders as an empty element (class of round-1 finding 1)', () => {
  for (const availability of AVAILABILITIES) {
    for (const kind of KINDS) {
      it(`${availability} ${kind} with every string field empty`, () => {
        const hollow: Source = {
          source_id: '',
          kind,
          location: '',
          availability,
          tool: '',
          tool_path: '',
          tool_version: '',
          diagnostic: '',
          summary: { branch: '', head: '', dirty: 0, roots: [], files: [], note: '' },
        }
        for (const [element, text] of leafTexts(hollow)) {
          expect(text, `${availability} ${kind}: ${element}`).not.toBe('')
        }
      })
    }
  }

  it('a detached HEAD shows "detached HEAD" as the branch', () => {
    const wrapper = mount(SourceCard, {
      props: {
        source: {
          source_id: 'vcs',
          kind: 'Vcs',
          location: '.',
          availability: 'Present',
          tool: 'git',
          tool_path: '/usr/bin/git',
          tool_version: 'git version 2.51.0',
          diagnostic: null,
          summary: { branch: '', head: 'abcdef0123', dirty: 0 },
        },
      },
    })
    const dts = wrapper.findAll('dt').map((dt) => dt.text())
    const dds = wrapper.findAll('dd').map((dd) => dd.text())
    expect(dds[dts.indexOf('branch')]).toBe('detached HEAD')
  })

  for (const kind of KINDS) {
    it(`${kind}: a summary field sent as null keeps its row, with a fallback and never "null"`, () => {
      const wrapper = mount(SourceCard, {
        props: {
          source: {
            source_id: 'src-x',
            kind,
            location: '.',
            availability: 'Present',
            tool: 'tool-x',
            tool_path: '/opt/x/tool-x',
            tool_version: 'version-x',
            diagnostic: null,
            summary: { branch: null, head: null, dirty: null, note: null },
          },
        },
      })
      const dts = wrapper.findAll('dt').map((dt) => dt.text())
      const dds = wrapper.findAll('dd').map((dd) => dd.text().trim())
      const expected =
        kind === 'Vcs' ? ['branch', 'head', 'worktree'] : ['branch', 'head', 'dirty', 'note']
      expect(dts).toEqual(expected)
      for (const value of dds) {
        expect(value).not.toBe('')
        expect(value).not.toBe('null')
      }
      if (kind === 'Vcs') expect(dds[dts.indexOf('head')]).toBe('no commit')
    })
  }
})

// Which tool fields a card shows, by availability. The server names the tool on every source that
// has one (crates/repoview-sources/src/lib.rs `blank`), whether or not it ran, so the tool line is
// only for states where the tool answered: Present and Failed. ToolMissing names the tool in its
// status; Absent shows no tool at all.
const TOOL_FIELDS: Record<Availability, 'ran' | 'named-in-status' | 'hidden'> = {
  Present: 'ran',
  Failed: 'ran',
  ToolMissing: 'named-in-status',
  Absent: 'hidden',
}

describe('every field the server sends is on the card, by the tool-field rule (rounds 1 and 2)', () => {
  for (const availability of AVAILABILITIES) {
    it(`${availability}: source fields always; tool fields ${TOOL_FIELDS[availability]}`, () => {
      const source: Source = {
        source_id: 'src-x',
        kind: 'Specification',
        location: 'loc-x',
        availability,
        tool: 'tool-x',
        tool_path: '/opt/x/tool-x',
        tool_version: 'version-x',
        diagnostic: 'diag-x',
        summary: { note: 'summary-x' },
      }
      const wrapper = mount(SourceCard, { props: { source } })
      const text = wrapper.text()
      for (const value of ['src-x', 'Specification', 'loc-x', 'diag-x', 'summary-x']) {
        expect(text, `${availability} card`).toContain(value)
      }
      switch (TOOL_FIELDS[availability]) {
        case 'ran':
          expect(wrapper.get('.tool').text()).toBe('tool-x · version-x')
          expect(wrapper.get('.tool-path').text()).toBe('/opt/x/tool-x')
          break
        case 'named-in-status':
          expect(wrapper.get('[data-test="source-status"]').text()).toBe('tool missing: tool-x')
          expect(wrapper.find('.tool').exists()).toBe(false)
          expect(wrapper.find('.tool-path').exists()).toBe(false)
          expect(text).not.toContain('version-x')
          break
        case 'hidden':
          expect(wrapper.find('.tool').exists()).toBe(false)
          expect(wrapper.find('.tool-path').exists()).toBe(false)
          for (const value of ['tool-x', 'version-x']) expect(text).not.toContain(value)
          break
      }
    })
  }

  it('an Absent card as the server sends it (tool named, nothing ran) shows no tool line', () => {
    const wrapper = mount(SourceCard, {
      props: {
        source: {
          source_id: 'quality',
          kind: 'Quality',
          location: '.',
          availability: 'Absent',
          tool: 'codegate',
          tool_path: null,
          tool_version: null,
          diagnostic: null,
          summary: {},
        },
      },
    })
    expect(wrapper.get('[data-test="source-status"]').text()).toBe('absent')
    expect(wrapper.text()).not.toContain('codegate')
    expect(wrapper.text()).not.toContain('version unknown')
  })

  it('a ToolMissing card as the server sends it shows no "· version unknown" line', () => {
    const wrapper = mount(SourceCard, {
      props: {
        source: {
          source_id: 'plan',
          kind: 'Planning',
          location: '.engineering',
          availability: 'ToolMissing',
          tool: 'aep',
          tool_path: null,
          tool_version: null,
          diagnostic: 'aep not found on PATH',
          summary: {},
        },
      },
    })
    expect(wrapper.get('[data-test="source-status"]').text()).toBe('tool missing: aep')
    expect(wrapper.text()).not.toContain('·')
    expect(wrapper.text()).not.toContain('version unknown')
  })

  it('a Failed card shows tool and tool_version alongside "failed" and the diagnostic', () => {
    const wrapper = mount(SourceCard, {
      props: {
        source: {
          source_id: 'spec',
          kind: 'Specification',
          location: 'ess',
          availability: 'Failed',
          tool: 'ess',
          tool_path: '/usr/local/bin/ess',
          tool_version: 'ess 0.52.0',
          diagnostic: 'boom',
          summary: {},
        },
      },
    })
    expect(wrapper.get('[data-test="source-status"]').text()).toBe('failed')
    expect(wrapper.get('.tool').text()).toContain('ess · ess 0.52.0')
    expect(wrapper.get('.diagnostic').text()).toBe('boom')
  })
})
