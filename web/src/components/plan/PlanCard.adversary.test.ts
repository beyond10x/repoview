import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import { createAppRouter } from '../../router'
import PlanCard from './PlanCard.vue'

// Adversary, story:plan-pages. `aep plan artifact board|list --format json` (aep 0.68.0) prints
// `blocked_by` as objects, not ids. Captured from the aep repository's own store:
//   "blocked_by":[{"blocker":"dependency-blocker:metaharness-source-receipt","type":"dependency"}]
const BLOCKED = {
  id: 'migration-plan:foundation-source-pins-20260909',
  kind: 'migration-plan',
  status: 'active',
  title: 'Foundation source pins',
  relations: [],
  refs: [],
  blocked_by: [{ blocker: 'dependency-blocker:metaharness-source-receipt', type: 'dependency' }],
}

describe('a blocked card as aep prints it', () => {
  it('names the blocker, not [object Object]', async () => {
    const router = createAppRouter(createMemoryHistory())
    await router.push('/plan')
    await router.isReady()
    const wrapper = mount(PlanCard, {
      // The real document, not the interface the unit declared for it.
      props: { artifact: BLOCKED as never },
      global: { plugins: [router] },
    })
    expect(wrapper.text()).not.toContain('[object Object]')
    expect(wrapper.text()).toContain('dependency-blocker:metaharness-source-receipt')
  })
})
