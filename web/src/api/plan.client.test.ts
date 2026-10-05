import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TOKEN_STORAGE_KEY, forgetToken } from './client'
import { failureText, loadArtifact, loadValidate, toolFailure, validation } from './plan'

// story:plan-pages acceptances 4 and 5 through the real apiGet: the server's 502 body reaches the
// page.

function serve(status: number, body: unknown) {
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

beforeEach(() => {
  forgetToken()
  sessionStorage.clear()
  sessionStorage.setItem(TOKEN_STORAGE_KEY, 'c'.repeat(64))
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('a failed aep reaches the plan pages', () => {
  it('a 502 on show yields its stderr', async () => {
    const body = { tool: 'aep', exit: 1, stderr: 'error: the store holds no `story:x`\n' }
    serve(502, body)
    const result = await loadArtifact('story:x')
    expect(toolFailure(result)).toEqual(body)
    expect(failureText(result)).toBe(body.stderr)
  })

  it('a 502 on validate yields the problems aep printed', async () => {
    const problems = ['story/x.md: declares kind storyy']
    serve(502, { tool: 'aep', exit: 1, stderr: '', stdout: JSON.stringify({ problems }) })
    expect(validation(await loadValidate())).toEqual({ state: 'problems', lines: problems })
  })
})
