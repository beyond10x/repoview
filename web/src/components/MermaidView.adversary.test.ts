import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import MermaidView from './MermaidView.vue'

// Adversary, story:page-frame acceptance 5, against the real Mermaid (not mocked). Mermaid 12.1.0
// `render` draws its "Syntax error" diagram into a temporary `<div id="d<id>">` on document.body and
// throws before removing it unless `suppressErrorRendering` is set. MermaidView shows its own error
// <pre>, so the bomb diagram is a second, stray copy left at the bottom of the page, one per failure.

afterEach(() => {
  document.body.innerHTML = ''
})

describe('MermaidView with the real Mermaid', () => {
  it('a parse error shows the <pre> and leaves nothing of Mermaid outside the component', async () => {
    const host = document.createElement('div')
    document.body.append(host)
    const wrapper = mount(MermaidView, {
      props: { source: 'graph TD\n  A -->' },
      attachTo: host,
    })
    // Mermaid is imported lazily; give the import and the render time to settle.
    for (let i = 0; i < 50 && !wrapper.find('pre').exists(); i++) {
      await flushPromises()
      await new Promise((resolve) => setTimeout(resolve, 20))
    }
    expect(wrapper.find('pre').exists()).toBe(true)

    const outside = [...document.body.children].filter((el) => el !== host)
    expect(outside.map((el) => `${el.tagName.toLowerCase()}#${el.id}`)).toEqual([])
    expect(document.body.querySelectorAll('[id^="drepoview-mermaid-"]')).toHaveLength(0)
    wrapper.unmount()
  }, 30_000)
})
