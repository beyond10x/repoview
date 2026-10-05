import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { LanguageQuality } from '../../api/quality'
import LanguageCard from './LanguageCard.vue'

// Adversary case, story:quality-page acceptance 3: the assessment is codegate's document passed
// through, and `web/src/api/quality.ts` declares every field of it optional. A JSON `null` is how
// an optional field arrives from a serialiser that does not omit it.

describe('a top finding whose location is null', () => {
  it('still renders the card and the finding title, without a location', () => {
    const entry = {
      language: 'go',
      status: 'assessed',
      reason: null,
      assessment: {
        rating: 'A',
        top_findings: [{ kind: 'k', reason: 'why', location: null }],
      },
    } as unknown as LanguageQuality
    const wrapper = mount(LanguageCard, { props: { entry } })
    expect(wrapper.get('[data-test="finding-title"]').text()).toBe('why')
    expect(wrapper.find('[data-test="finding-location"]').exists()).toBe(false)
  })
})
