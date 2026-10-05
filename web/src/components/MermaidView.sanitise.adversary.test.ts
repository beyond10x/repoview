import { flushPromises, mount } from '@vue/test-utils'
import mermaid from 'mermaid'
import { afterEach, describe, expect, it, vi } from 'vitest'
import MermaidView from './MermaidView.vue'

// Adversary, story:page-frame acceptance 5: the second DOMPurify pass, fed SVG that a Mermaid
// bypass would produce. Mermaid is mocked so the pass is the only defence under test.

vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: vi.fn() } }))

afterEach(() => {
  vi.mocked(mermaid.render).mockReset()
})

async function renderSvg(svg: string) {
  vi.mocked(mermaid.render).mockResolvedValue({ svg, diagramType: 'flowchart' })
  const wrapper = mount(MermaidView, { props: { source: 'graph TD\n  A --> B' } })
  await flushPromises()
  await flushPromises()
  return wrapper
}

const SCRIPTY = /^\s*(javascript|vbscript|data:text\/html)/i

describe('MermaidView second sanitisation pass', () => {
  it('drops script URLs, animation-driven hrefs and handlers inside foreignObject', async () => {
    const wrapper = await renderSvg(
      [
        '<svg data-test="diagram">',
        '<a href="javascript:alert(1)"><text>a</text></a>',
        '<a xlink:href="javascript:alert(1)"><text>b</text></a>',
        '<a><set attributeName="href" to="javascript:alert(1)"/><text>c</text></a>',
        '<a><animate attributeName="href" values="javascript:alert(1)"/><text>d</text></a>',
        '<use href="data:image/svg+xml;base64,PHN2ZyBvbmxvYWQ9YWxlcnQoMSk+"/>',
        '<foreignObject><div><img src="x" onerror="alert(1)"><iframe src="javascript:alert(1)"></iframe>',
        '<a href="javascript:alert(1)">e</a><script>alert(1)</script></div></foreignObject>',
        '<foreignObject><svg><foreignObject><math><mi><style><img src=x onerror=alert(1)></style></mi></math></foreignObject></svg></foreignObject>',
        '</svg>',
      ].join(''),
    )
    const root = wrapper.element
    expect(root.querySelector('svg[data-test="diagram"]')).not.toBeNull()
    const all = [...root.querySelectorAll('*')]
    const urls = all.flatMap((el) =>
      [...el.attributes]
        .filter((a) => /(^|:)href$|^src$|^to$|^values$/i.test(a.name))
        .map((a) => a.value),
    )
    expect(urls.filter((u) => SCRIPTY.test(u))).toEqual([])
    expect(
      all.flatMap((el) => [...el.attributes].map((a) => a.name)).filter((n) => /^on/i.test(n)),
    ).toEqual([])
    expect(root.querySelectorAll('script, iframe, set, animate')).toHaveLength(0)
  })

  it("keeps what a diagram needs to look like one: Mermaid's <style> and HTML labels", async () => {
    const wrapper = await renderSvg(
      '<svg id="m1" data-test="diagram"><style>#m1 .node rect{fill:#eee}</style>' +
        '<g class="node"><rect/><foreignObject width="40" height="20">' +
        '<div xmlns="http://www.w3.org/1999/xhtml"><span class="nodeLabel"><p>Draft</p></span></div>' +
        '</foreignObject></g></svg>',
    )
    const svg = wrapper.get('svg[data-test="diagram"]').element
    expect(svg.querySelector('style')?.textContent).toContain('fill:#eee')
    expect(svg.querySelector('foreignObject span.nodeLabel p')?.textContent).toBe('Draft')
  })
})
