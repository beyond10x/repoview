// Payloads of the Repository page's routes, shaped as the server answers them.

import type { DocumentEntry, Vcs } from '../../api/repository'

export const HEAD = '9f3c2b1a0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b'
export const PARENT = '1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b'

export const vcsFull: Vcs = {
  branch: 'main',
  head: HEAD,
  upstream: 'origin/main',
  ahead: 2,
  behind: 1,
  dirty: [
    { path: 'src/lib.rs', status: '.M', orig_path: null },
    { path: 'src/new.rs', status: 'A.', orig_path: null },
    { path: 'docs/guide.md', status: 'R.', orig_path: 'docs/old-guide.md' },
    { path: 'conflict.txt', status: 'UU', orig_path: null },
    { path: 'scratch.txt', status: '??', orig_path: null },
  ],
  commits: [
    {
      sha: HEAD,
      author: 'Ada Lovelace',
      date: '2026-10-05T14:03:11+02:00',
      subject: 'feat: the repository page',
    },
    {
      sha: PARENT,
      author: 'Grace Hopper',
      date: '2026-10-04T09:30:00+02:00',
      subject: 'fix: <b>not bold</b>',
    },
  ],
  tags: [
    { name: '0.2.0', sha: HEAD, date: '2026-10-05T14:05:00+02:00' },
    { name: '0.1.0', sha: PARENT, date: '2026-10-01T10:00:00+02:00' },
  ],
  remotes: [{ name: 'origin', url: 'https://github.com/o/r.git' }],
  worktrees: [
    { path: '/work/example', head: HEAD, branch: 'main', locked: false, prunable: false },
    {
      path: '/work/example-feature',
      head: PARENT,
      branch: 'feature',
      locked: true,
      prunable: false,
    },
    { path: '/work/example-gone', head: PARENT, branch: null, locked: false, prunable: true },
  ],
  tags_error: null,
  remotes_error: null,
  worktrees_error: null,
}

export const vcsDetached: Vcs = {
  ...vcsFull,
  branch: null,
  upstream: null,
  ahead: null,
  behind: null,
  dirty: [],
  worktrees: [
    { path: '/work/example', head: PARENT, branch: null, locked: false, prunable: false },
  ],
  head: PARENT,
}

/** `git init` and nothing else: an unborn `main`, no commits, tags, remotes or changes. */
export const vcsEmpty: Vcs = {
  branch: 'main',
  head: null,
  upstream: null,
  ahead: null,
  behind: null,
  dirty: [],
  commits: [],
  tags: [],
  remotes: [],
  worktrees: [{ path: '/work/empty', head: null, branch: 'main', locked: false, prunable: false }],
  tags_error: null,
  remotes_error: null,
  worktrees_error: null,
}

/** Git answered status and commits, but `git worktree list` failed (e.g. a Git too old for it). */
export const vcsWorktreesFailed: Vcs = {
  ...vcsFull,
  worktrees: [],
  worktrees_error: "error: unknown switch `z'",
}

/** An upstream whose remote-tracking branch is gone: named, without counts. */
export const vcsGoneUpstream: Vcs = { ...vcsFull, ahead: null, behind: null }

export const documents: DocumentEntry[] = [
  { name: 'README.md', present: true },
  { name: 'AGENTS.md', present: true },
  { name: 'STATUS.md', present: false },
  { name: 'CHANGELOG.md', present: true },
]

export const noDocuments: DocumentEntry[] = documents.map((entry) => ({ ...entry, present: false }))

export const documentBodies: Record<string, string> = {
  'README.md':
    '# Example\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n<script>window.pwned = 1</script>\n',
  'AGENTS.md': '# Agents\n\n```console\ntask check\n```\n',
  'CHANGELOG.md': '# Changelog\n\n## 0.2.0\n',
}
