import { describe, expect, it } from 'vitest'
import { createMemoryHistory } from 'vue-router'
import { createAppRouter } from '../../router'
import { artifactRoute } from './links'

// story:plan-pages acceptance 4: relation links reach the artifact they name, whatever its id holds.

describe('artifactRoute', () => {
  const router = createAppRouter(createMemoryHistory())

  it('keeps the colon readable', () => {
    expect(router.resolve(artifactRoute('vision:O2')).href).toBe('/plan/artifact/vision:O2')
  })

  it('encodes what would leave the path segment, and the id comes back unchanged', () => {
    for (const id of ['a/b:c', 'a#b:c', 'a?b:c', 'a%b:c', 'a b:c', 'a.b:c..d']) {
      const href = router.resolve(artifactRoute(id)).href
      expect(href.slice('/plan/artifact/'.length), id).not.toMatch(/[/#? ]/)
      const back = router.resolve(href)
      expect(back.name, id).toBe('plan-artifact')
      expect(back.params.id, id).toBe(id)
    }
  })
})
