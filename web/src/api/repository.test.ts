import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { vcsFull } from '../__fixtures__/repository'
import { serveRoutes } from '../__fixtures__/repository/fetch'
import {
  loadDocument,
  loadDocuments,
  loadVcs,
  normaliseDocuments,
  normaliseVcs,
  shortDate,
  shortSha,
} from './repository'

// story:repository-page acceptance 1 (the wire shapes) as the SPA reads them.

beforeEach(() => {
  sessionStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('normalisers', () => {
  it('a well-formed vcs payload is unchanged', () => {
    expect(normaliseVcs(JSON.parse(JSON.stringify(vcsFull)))).toEqual(vcsFull)
  })

  it('a vcs payload missing everything becomes empty lists and nulls', () => {
    expect(normaliseVcs(null)).toEqual({
      branch: null,
      head: null,
      upstream: null,
      ahead: null,
      behind: null,
      dirty: [],
      commits: [],
      tags: [],
      remotes: [],
      worktrees: [],
      tags_error: null,
      remotes_error: null,
      worktrees_error: null,
    })
  })

  it('documents keep their names and presence; nameless entries are dropped', () => {
    expect(
      normaliseDocuments([{ name: 'README.md', present: true }, { present: true }, 'x']),
    ).toEqual([{ name: 'README.md', present: true }])
  })
})

describe('loaders', () => {
  it('loadVcs reads /api/vcs', async () => {
    serveRoutes({ '/api/vcs': vcsFull })
    expect(await loadVcs()).toEqual({ state: 'ready', data: vcsFull })
  })

  it('a 404 is absent, a 503 an error', async () => {
    serveRoutes({ '/api/vcs': { status: 503 } })
    expect(await loadVcs()).toMatchObject({ state: 'unavailable', reason: 'error' })
    expect(await loadDocuments()).toEqual({
      state: 'unavailable',
      reason: 'absent',
      message: 'no documents',
    })
  })

  it('loadDocument names the 413', async () => {
    serveRoutes({ '/api/docs/CHANGELOG.md': { status: 413 } })
    expect(await loadDocument('CHANGELOG.md')).toEqual({
      state: 'unavailable',
      reason: 'error',
      message: 'CHANGELOG.md is larger than 1 MiB and is not shown',
    })
  })
})

describe('formatting', () => {
  it('shortSha keeps eight characters', () => {
    expect(shortSha('0123456789abcdef')).toBe('01234567')
    expect(shortSha(null)).toBe('')
  })

  it('shortDate shows date and minute, anything else as given', () => {
    expect(shortDate('2026-10-05T14:03:11+02:00')).toBe('2026-10-05 14:03')
    expect(shortDate('yesterday')).toBe('yesterday')
  })
})
