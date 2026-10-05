import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { LanguageQuality } from '../../api/quality'
import LanguageCard from './LanguageCard.vue'

// Round 1, the class behind `location: null`: the assessment is codegate's document passed
// through, so any field of it, at any depth, may be null or of another type. The card renders
// what is usable and drops the rest; it never throws.

function assessed(assessment: unknown): LanguageQuality {
  return {
    language: 'go',
    status: 'assessed',
    reason: null,
    assessment,
  } as unknown as LanguageQuality
}

const NULLS = {
  rating: null,
  score_max: null,
  summary: null,
  scores: null,
  finding_counts: null,
  top_findings: null,
}

const WRONG_TYPES = {
  rating: 7,
  score_max: 'x',
  summary: 'abc',
  scores: 'abc',
  finding_counts: ['a'],
  top_findings: { kind: 'k' },
}

describe('an assessment whose fields are null or of another type', () => {
  for (const [name, assessment] of [
    ['null', NULLS],
    ['wrong types', WRONG_TYPES],
  ] as const) {
    it(`renders the card with nothing made up (${name})`, () => {
      const wrapper = mount(LanguageCard, { props: { entry: assessed(assessment) } })
      expect(wrapper.get('[data-test="rating"]').text()).toBe('–')
      expect(wrapper.find('[data-test="score"]').exists()).toBe(false)
      expect(wrapper.find('[data-test="summary"]').exists()).toBe(false)
      expect(wrapper.find('[data-test="finding-count"]').exists()).toBe(false)
      expect(wrapper.find('[data-test="top-finding"]').exists()).toBe(false)
    })
  }

  it('a top finding that is null, or whose fields are null, is skipped or shown bare', () => {
    const wrapper = mount(LanguageCard, {
      props: {
        entry: assessed({
          top_findings: [
            null,
            'text',
            {
              kind: null,
              title: null,
              reason: null,
              severity: null,
              location: { uri: null, range: null },
            },
            { reason: 'why', severity: 3, location: { uri: 'a.go', range: { start: null } } },
            { reason: 'deep', location: { uri: 'b.go', range: { start: { line: null } } } },
          ],
        }),
      },
    })
    const findings = wrapper.findAll('[data-test="top-finding"]')
    expect(findings.map((f) => f.get('[data-test="finding-title"]').text())).toEqual([
      'untitled finding',
      'why',
      'deep',
    ])
    expect(
      wrapper.findAll('[data-test="finding-location"]').map((location) => location.text()),
    ).toEqual(['a.go', 'b.go'])
    expect(wrapper.find('.severity').exists()).toBe(false)
  })
})
