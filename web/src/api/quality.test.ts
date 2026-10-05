import { afterEach, describe, expect, it, vi } from 'vitest'
import { codegate030 } from '../__fixtures__/quality/reports'
import { CODEGATE_REPOSITORY, loadQuality } from './quality'

// story:quality-codegate acceptance 2 and 3: the wire contract as the SPA reads it.

function serve(status: number, body: unknown): void {
  vi.stubGlobal(
    'fetch',
    vi.fn<typeof fetch>(() =>
      Promise.resolve(
        new Response(JSON.stringify(body), {
          status,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    ),
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('loadQuality', () => {
  it('a codegate report is ready, unchanged', async () => {
    serve(200, codegate030)
    expect(await loadQuality()).toEqual({ state: 'ready', report: codegate030 })
  })

  it('a report with no reason (an assessment present) is still a report', async () => {
    const report = { ...codegate030, reason: null, assessment: { status: 'running' } }
    serve(200, report)
    expect(await loadQuality()).toEqual({ state: 'ready', report })
  })

  it('the old per-language document is not a report', async () => {
    serve(200, { tool: 'codegate', tool_path: '/x/codegate', languages: [] })
    expect((await loadQuality()).state).toBe('error')
  })

  it.each([
    ['tool_version', 3],
    ['tool_path', null],
    ['skipped', '/x'],
    ['commands', ['evaluate', 7]],
    ['reason', 5],
  ])('a report whose %s is %j is an error', async (key, value) => {
    serve(200, { ...codegate030, [key]: value })
    const state = await loadQuality()
    expect(state).toEqual({
      state: 'error',
      status: null,
      message: 'unexpected /api/quality document',
    })
  })

  it('a 503 is unavailable with the server stderr as its message', async () => {
    serve(503, { tool: 'codegate', exit: null, stderr: 'beyond10x codegate not found on PATH' })
    expect(await loadQuality()).toEqual({
      state: 'unavailable',
      message: 'beyond10x codegate not found on PATH',
    })
  })

  it('a 503 without a stderr falls back to the contract text', async () => {
    serve(503, 'no json here')
    expect(await loadQuality()).toEqual({
      state: 'unavailable',
      message: 'beyond10x codegate not found on PATH',
    })
  })

  it('names the beyond10x codegate repository', () => {
    expect(CODEGATE_REPOSITORY).toBe('https://github.com/beyond10x/codegate')
  })
})
