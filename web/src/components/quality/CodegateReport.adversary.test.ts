import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { codegate030 } from '../../__fixtures__/quality/reports'
import type { QualityReport } from '../../api/quality'
import CodegateReport from './CodegateReport.vue'

// Adversary case of story:quality-page (a top finding whose location is null), rewritten by
// story:quality-codegate: the page no longer renders codegate's assessment document at all, so an
// assessment shaped like the Go adapter's, null fields included, must neither break the report
// nor reach the screen as a rating or a finding.

describe('an assessment shaped like the Go adapter document', () => {
  it('renders the report and none of the assessment, without failing on its null fields', () => {
    const report = {
      ...codegate030,
      reason: null,
      assessment: {
        rating: 'A',
        top_findings: [{ kind: 'k', reason: 'why', location: null }],
      },
    } as QualityReport
    const wrapper = mount(CodegateReport, { props: { report } })
    expect(wrapper.get('[data-test="tool-version"]').text()).toBe('0.3.0')
    expect(wrapper.find('[data-test="finding-title"]').exists()).toBe(false)
    expect(wrapper.find('[data-test="rating"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('why')
    expect(wrapper.find('[data-test="reason"]').exists()).toBe(false)
  })
})
