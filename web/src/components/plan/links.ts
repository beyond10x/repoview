import type { RouteLocationRaw } from 'vue-router'

/**
 * The Artifact page of `id`, by route name, so vue-router encodes the id as a path parameter:
 * `/`, `#`, `?`, `%` and spaces are escaped, and the id comes back unchanged as `params.id`.
 */
export function artifactRoute(id: string): RouteLocationRaw {
  return { name: 'plan-artifact', params: { id } }
}
