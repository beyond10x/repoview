// The left nav, and the page each snapshot source belongs to.

/** One left-nav entry: its page, and the snapshot sources (by `source_id`) that page reads. */
export interface NavEntry {
  title: string
  to: string
  sources: string[]
}

export const NAV: readonly NavEntry[] = [
  { title: 'Overview', to: '/', sources: [] },
  { title: 'Plan', to: '/plan', sources: ['plan'] },
  { title: 'Specs', to: '/specs', sources: ['spec'] },
  { title: 'Quality', to: '/quality', sources: ['quality'] },
  { title: 'Repository', to: '/repository', sources: ['vcs', 'docs'] },
]

/** The page that shows a source, or `null` when no page does. */
export function pageForSource(sourceId: string): string | null {
  return NAV.find((entry) => entry.sources.includes(sourceId))?.to ?? null
}
