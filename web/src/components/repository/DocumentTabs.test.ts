import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { vi } from 'vitest'
import { documentBodies, documents, noDocuments } from '../../__fixtures__/repository'
import { requestedUrls, serveRoutes } from '../../__fixtures__/repository/fetch'
import MarkdownView from '../MarkdownView.vue'
import DocumentTabs from './DocumentTabs.vue'

// story:repository-page acceptance 2 and 4: a tab per present document, rendered through
// MarkdownView; absent documents listed as "absent".

function serveDocuments(extra: Record<string, unknown> = {}) {
  return serveRoutes({
    '/api/docs/README.md': { name: 'README.md', markdown: documentBodies['README.md'] },
    '/api/docs/AGENTS.md': { name: 'AGENTS.md', markdown: documentBodies['AGENTS.md'] },
    '/api/docs/CHANGELOG.md': { name: 'CHANGELOG.md', markdown: documentBodies['CHANGELOG.md'] },
    ...extra,
  })
}

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('DocumentTabs', () => {
  it('has a tab per present document and lists absent ones as absent', async () => {
    serveDocuments()
    const wrapper = mount(DocumentTabs, { props: { documents } })
    await flushPromises()
    const tabs = wrapper.findAll('[data-test="doc-tab"]')
    expect(tabs.map((tab) => tab.text())).toEqual(['README.md', 'AGENTS.md', 'CHANGELOG.md'])
    expect(tabs.map((tab) => tab.attributes('role'))).toEqual(['tab', 'tab', 'tab'])
    const absent = wrapper.findAll('[data-test="doc-absent"]')
    expect(absent.map((entry) => entry.text())).toEqual(['STATUS.md absent'])
  })

  it('opens on the first present document, rendered through MarkdownView', async () => {
    serveDocuments()
    const wrapper = mount(DocumentTabs, { props: { documents } })
    await flushPromises()
    expect(wrapper.get('[data-test="doc-tab"][aria-selected="true"]').text()).toBe('README.md')
    const view = wrapper.getComponent(MarkdownView)
    expect(view.props('source')).toBe(documentBodies['README.md'])
    const panel = wrapper.get('[data-test="doc-panel"]')
    expect(panel.find('table').exists()).toBe(true)
    expect(panel.element.querySelectorAll('script')).toHaveLength(0)
  })

  it('selecting a tab loads and renders that document once', async () => {
    const mock = serveDocuments()
    const wrapper = mount(DocumentTabs, { props: { documents } })
    await flushPromises()
    const agents = wrapper.findAll('[data-test="doc-tab"]')[1]
    if (agents === undefined) throw new Error('no AGENTS.md tab')
    await agents.trigger('click')
    await flushPromises()
    expect(agents.attributes('aria-selected')).toBe('true')
    expect(wrapper.get('[data-test="doc-panel"] pre code').text()).toBe('task check')
    await wrapper.findAll('[data-test="doc-tab"]')[0]?.trigger('click')
    await agents.trigger('click')
    await flushPromises()
    expect(requestedUrls(mock).filter((url) => url === '/api/docs/AGENTS.md')).toHaveLength(1)
  })

  it('a document over 1 MiB says so instead of rendering', async () => {
    serveDocuments({ '/api/docs/README.md': { status: 413, body: { limit: 1048576 } } })
    const wrapper = mount(DocumentTabs, { props: { documents } })
    await flushPromises()
    expect(wrapper.get('[data-test="doc-panel"]').text()).toBe(
      'README.md is larger than 1 MiB and is not shown',
    )
    expect(wrapper.findComponent(MarkdownView).exists()).toBe(false)
  })

  it('no present document: every one is listed absent and nothing is fetched', async () => {
    const mock = serveDocuments()
    const wrapper = mount(DocumentTabs, { props: { documents: noDocuments } })
    await flushPromises()
    expect(wrapper.findAll('[data-test="doc-tab"]')).toHaveLength(0)
    expect(wrapper.findAll('[data-test="doc-absent"]')).toHaveLength(4)
    expect(wrapper.find('[data-test="doc-panel"]').exists()).toBe(false)
    expect(mock).not.toHaveBeenCalled()
  })
})
