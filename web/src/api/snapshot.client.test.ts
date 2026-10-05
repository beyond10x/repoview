import { afterEach, describe, expect, it, vi } from 'vitest'
import { allPresent } from '../__fixtures__/snapshots'
import { apiGet } from './client'
import { loadSnapshot } from './snapshot'

// story:page-frame acceptance 2: snapshot.ts reads through apiGet.

vi.mock('./client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./client')>()
  return { ...actual, apiGet: vi.fn() }
})

afterEach(() => {
  vi.mocked(apiGet).mockReset()
})

describe('loadSnapshot through apiGet', () => {
  it('asks apiGet for "snapshot" and answers its data as the snapshot', async () => {
    vi.mocked(apiGet).mockResolvedValue({ state: 'ready', data: allPresent })
    expect(await loadSnapshot()).toEqual({ state: 'ready', snapshot: allPresent })
    expect(apiGet).toHaveBeenCalledExactlyOnceWith('snapshot')
  })

  it('passes token-rejected through and prefixes an error', async () => {
    vi.mocked(apiGet).mockResolvedValue({ state: 'token-rejected' })
    expect(await loadSnapshot()).toEqual({ state: 'token-rejected' })
    vi.mocked(apiGet).mockResolvedValue({ state: 'error', status: 500, message: 'HTTP 500' })
    expect(await loadSnapshot()).toEqual({
      state: 'error',
      message: 'snapshot unavailable: HTTP 500',
    })
  })
})
