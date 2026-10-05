import { flushPromises, mount } from '@vue/test-utils'
import mermaid from 'mermaid'
import { afterEach, describe, expect, it, vi } from 'vitest'
import MermaidView from './MermaidView.vue'

// story:page-frame acceptance 5. Mermaid needs a real layout engine, so it is mocked here.

vi.mock('mermaid', () => ({
  default: {
    initialize: vi.fn(),
    render: vi.fn(),
  },
}))

const SOURCE = 'stateDiagram-v2\n  [*] --> Draft\n  Draft --> Active'

async function render(source: string) {
  const wrapper = mount(MermaidView, { props: { source } })
  await flushPromises()
  return wrapper
}

afterEach(() => {
  vi.mocked(mermaid.render).mockReset()
  vi.mocked(mermaid.initialize).mockClear()
})

describe('MermaidView', () => {
  it('initialises Mermaid with securityLevel "strict" and renders the SVG', async () => {
    vi.mocked(mermaid.render).mockResolvedValue({
      svg: '<svg data-test="diagram"><g><text>Draft</text></g></svg>',
      diagramType: 'stateDiagram',
    })
    const wrapper = await render(SOURCE)
    expect(mermaid.initialize).toHaveBeenCalledWith(
      expect.objectContaining({ securityLevel: 'strict', startOnLoad: false }),
    )
    expect(vi.mocked(mermaid.render).mock.calls[0]?.[1]).toBe(SOURCE)
    expect(wrapper.find('svg[data-test="diagram"]').exists()).toBe(true)
    expect(wrapper.find('pre').exists()).toBe(false)
  })

  it('a render error shows the error text and the source in a <pre>, never a blank box', async () => {
    vi.mocked(mermaid.render).mockRejectedValue(new Error('Parse error on line 2'))
    const wrapper = await render(SOURCE)
    const pre = wrapper.get('pre')
    expect(pre.text()).toContain('Parse error on line 2')
    expect(pre.text()).toContain('Draft --> Active')
    expect(wrapper.find('svg').exists()).toBe(false)
    expect(wrapper.text().trim()).not.toBe('')
  })

  it('a non-Error rejection or a synchronous throw is still shown', async () => {
    vi.mocked(mermaid.render).mockRejectedValue('bad diagram')
    expect((await render(SOURCE)).get('pre').text()).toContain('bad diagram')

    vi.mocked(mermaid.render).mockImplementation(() => {
      throw new Error('sync failure')
    })
    expect((await render(SOURCE)).get('pre').text()).toContain('sync failure')
  })

  it('an empty source is shown as an error, not a blank box, and is not rendered', async () => {
    const wrapper = await render('   ')
    expect(mermaid.render).not.toHaveBeenCalled()
    expect(wrapper.get('pre').text()).toContain('empty diagram source')
  })

  it('shows a notice while rendering', () => {
    vi.mocked(mermaid.render).mockReturnValue(new Promise(() => undefined))
    const wrapper = mount(MermaidView, { props: { source: SOURCE } })
    expect(wrapper.text()).toContain('rendering diagram')
  })

  it('strips script and event handlers from the SVG it is given', async () => {
    vi.mocked(mermaid.render).mockResolvedValue({
      svg: '<svg data-test="diagram" onload="window.pwned=1"><script>window.pwned=1</script><g onclick="x()"><foreignObject><div class="label">Draft</div></foreignObject></g></svg>',
      diagramType: 'flowchart',
    })
    const wrapper = await render(SOURCE)
    expect(wrapper.find('svg[data-test="diagram"]').exists()).toBe(true)
    expect(wrapper.element.querySelectorAll('script')).toHaveLength(0)
    expect(wrapper.element.querySelectorAll('[onload], [onclick]')).toHaveLength(0)
    expect(wrapper.text()).toContain('Draft')
  })

  it('re-renders when the source changes', async () => {
    vi.mocked(mermaid.render).mockResolvedValue({ svg: '<svg></svg>', diagramType: 'x' })
    const wrapper = await render(SOURCE)
    await wrapper.setProps({ source: 'graph TD\n  A --> B' })
    await flushPromises()
    expect(vi.mocked(mermaid.render).mock.calls.map((call) => call[1])).toEqual([
      SOURCE,
      'graph TD\n  A --> B',
    ])
  })
})
