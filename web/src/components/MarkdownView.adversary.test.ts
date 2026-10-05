import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import MarkdownView from './MarkdownView.vue'

// Adversary, story:page-frame acceptance 4: URL schemes and markup the shipped tests do not try.

function render(source: string) {
  return mount(MarkdownView, { props: { source } })
}

const DANGEROUS = /^\s*(javascript|vbscript|data:text\/html|data:image\/svg|data:application)/i

function urls(root: Element): string[] {
  return [...root.querySelectorAll('*')].flatMap((el) =>
    [...el.attributes]
      .filter((a) => ['href', 'src', 'xlink:href', 'action', 'formaction'].includes(a.name))
      .map((a) => a.value),
  )
}

describe('MarkdownView, further hostile input', () => {
  it('no vbscript:, data:text/html or data:image/svg URL survives, in any link form', () => {
    const wrapper = render(
      [
        '[a](vbscript:msgbox(1))',
        '[b](data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==)',
        '![c](data:image/svg+xml;base64,PHN2ZyBvbmxvYWQ9YWxlcnQoMSk+)',
        '<data:text/html,<script>alert(1)</script>>',
        '[d][ref]\n\n[ref]: javascript:alert(1)',
        '[e](&#106;avascript:alert(1))',
        '[f](java&#x09;script:alert(1))',
        '[g](%6Aavascript:alert(1))',
        'www.example.com/"onmouseover="alert(1)',
        'https://example.com/"><img src=x onerror=alert(1)>',
      ].join('\n\n'),
    )
    expect(urls(wrapper.element).filter((u) => DANGEROUS.test(u))).toEqual([])
    const names = [...wrapper.element.querySelectorAll('*')].flatMap((el) =>
      [...el.attributes].map((a) => a.name),
    )
    expect(names.filter((n) => /^on/i.test(n))).toEqual([])
    expect(wrapper.element.querySelectorAll('img[onerror], script')).toHaveLength(0)
  })

  it('raw SVG, MathML, form and style markup does not become elements', () => {
    const wrapper = render(
      [
        '<svg><a xlink:href="javascript:alert(1)"><text>x</text></a></svg>',
        '<math><mtext><table><mglyph><style><img src=x onerror=alert(1)>',
        '<form action="https://evil.example"><input name="token"></form>',
        '<style>body{display:none}</style>',
        '<iframe src="https://evil.example"></iframe>',
      ].join('\n\n'),
    )
    expect(
      wrapper.element.querySelectorAll('svg, math, form, input, style, iframe, img'),
    ).toHaveLength(0)
  })

  it('a fence info string cannot add attributes or classes beyond language-<word>', () => {
    const wrapper = render('```x" onclick="alert(1) class="evil\ncode\n```\n')
    const code = wrapper.get('pre > code')
    expect(code.attributes('onclick')).toBeUndefined()
    expect(code.classes().every((c) => c.startsWith('language-'))).toBe(true)
  })
})
