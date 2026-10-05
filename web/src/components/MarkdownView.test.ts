import { mount } from '@vue/test-utils'
import DOMPurify from 'dompurify'
import { afterEach, describe, expect, it, vi } from 'vitest'
import MarkdownView from './MarkdownView.vue'

// story:page-frame acceptance 4.

function render(source: string) {
  return mount(MarkdownView, { props: { source } })
}

// Every attribute name on the element and its descendants.
function attributeNames(root: Element): string[] {
  return [root, ...root.querySelectorAll('*')].flatMap((el) =>
    [...el.attributes].map((a) => a.name),
  )
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('MarkdownView keeps hostile markup out of the DOM', () => {
  it('a <script> tag does not appear', () => {
    const wrapper = render('before\n\n<script>window.pwned = 1</script>\n\nafter')
    expect(wrapper.element.querySelectorAll('script')).toHaveLength(0)
    expect(wrapper.html()).not.toMatch(/<script/i)
  })

  it('an onerror= attribute does not appear', () => {
    const wrapper = render(
      '<img src="x" onerror="window.pwned = 1">\n\n![a](x "t\\" onerror=\\"y")',
    )
    expect(wrapper.element.querySelectorAll('[onerror]')).toHaveLength(0)
    expect(attributeNames(wrapper.element).filter((name) => /^on/i.test(name))).toEqual([])
  })

  it('a javascript: link does not appear', () => {
    const wrapper = render(
      [
        '[a](javascript:alert(1))',
        '[b](JaVaScRiPt:alert(1))',
        '[c](  javascript:alert(1))',
        '<javascript:alert(1)>',
        '<a href="javascript:alert(1)">d</a>',
      ].join('\n\n'),
    )
    for (const a of wrapper.element.querySelectorAll('a')) {
      expect(a.getAttribute('href') ?? '').not.toMatch(/^\s*javascript:/i)
    }
    for (const el of wrapper.element.querySelectorAll('[href], [src]')) {
      for (const name of ['href', 'src']) {
        expect(el.getAttribute(name) ?? '').not.toMatch(/^\s*javascript:/i)
      }
    }
  })

  it('the rendered HTML passes through DOMPurify before it reaches the DOM', () => {
    const sanitize = vi.spyOn(DOMPurify, 'sanitize').mockReturnValue('<p data-x="purified">ok</p>')
    const wrapper = render('# Title')
    expect(sanitize).toHaveBeenCalled()
    expect(sanitize.mock.calls[0]?.[0]).toContain('<h1>Title</h1>')
    expect(wrapper.find('[data-x="purified"]').exists()).toBe(true)
    expect(wrapper.find('h1').exists()).toBe(false)
  })
})

describe('MarkdownView renders markdown', () => {
  it('a table appears', () => {
    const wrapper = render('| a | b |\n|---|---|\n| 1 | 2 |\n')
    const table = wrapper.get('table')
    expect(table.findAll('th').map((th) => th.text())).toEqual(['a', 'b'])
    expect(table.findAll('td').map((td) => td.text())).toEqual(['1', '2'])
  })

  it('a fenced code block appears, with its language class', () => {
    const wrapper = render('```rust\nfn main() {}\n```\n')
    const code = wrapper.get('pre > code')
    expect(code.text()).toBe('fn main() {}')
    expect(code.classes()).toContain('language-rust')
  })

  it('linkifies a bare URL', () => {
    const wrapper = render('see https://example.com/x for more')
    expect(wrapper.get('a').attributes('href')).toBe('https://example.com/x')
  })

  it('re-renders when the source changes', async () => {
    const wrapper = render('# one')
    await wrapper.setProps({ source: '# two' })
    expect(wrapper.get('h1').text()).toBe('two')
  })
})

describe('v-html', () => {
  it('appears in no component but MarkdownView', () => {
    const sources = import.meta.glob<string>('/src/**/*.vue', {
      query: '?raw',
      import: 'default',
      eager: true,
    })
    const files = Object.keys(sources)
    const isMarkdownView = (file: string) => file === '/src/components/MarkdownView.vue'
    expect(files.filter(isMarkdownView)).toHaveLength(1)
    const offenders = files.filter(
      (file) => !isMarkdownView(file) && /\bv-html\b/.test(sources[file] ?? ''),
    )
    expect(offenders).toEqual([])
  })
})
