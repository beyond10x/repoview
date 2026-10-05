// Wire contract: story:repository-page acceptance 1 (`/api/vcs`, `/api/docs`, `/api/docs/{name}`)
// and story:taskfile-tasks acceptance 1 (`/api/tasks`).
// Every payload is normalised here, so a block never meets a missing array.

import { apiGet, type ApiResult } from './client'
import { TOKEN_REJECTED_MESSAGE } from './snapshot'

export interface DirtyFile {
  path: string
  /** The porcelain v2 `XY` letters (`.M`, `A.`, `UU`, `??`), as Git printed them. */
  status: string
  /** The path before a rename or copy. */
  orig_path: string | null
}

export interface Commit {
  sha: string
  author: string
  date: string
  subject: string
}

export interface Tag {
  name: string
  sha: string
  date: string
}

export interface Remote {
  name: string
  url: string
}

export interface Worktree {
  path: string
  head: string | null
  branch: string | null
  locked: boolean
  prunable: boolean
}

export interface Vcs {
  /** `null` on a detached HEAD. */
  branch: string | null
  /** `null` before the first commit. */
  head: string | null
  upstream: string | null
  /** `null` without an upstream, or when the upstream ref is gone. */
  ahead: number | null
  behind: number | null
  dirty: DirtyFile[]
  commits: Commit[]
  tags: Tag[]
  remotes: Remote[]
  worktrees: Worktree[]
  /** Why Git could not list tags, remotes or worktrees; `null` when it could. The list is then empty. */
  tags_error: string | null
  remotes_error: string | null
  worktrees_error: string | null
}

export interface DocumentEntry {
  name: string
  present: boolean
}

export interface Document {
  name: string
  markdown: string
}

/** One task from the Taskfile, as written there; nothing in it was evaluated. */
export interface TaskEntry {
  /** Prefixed with its include namespaces, `docs:build`. */
  name: string
  desc: string | null
  summary: string | null
  /** The task's own `internal: true`, or that of an include above it. */
  internal: boolean
  aliases: string[]
}

/** An `includes:` entry the server did not read, and why. */
export interface RefusedInclude {
  /** The include's namespace, `docs` or `docs:api`. */
  include: string
  /** The path as the Taskfile wrote it; `null` when it named none. */
  taskfile: string | null
  reason: string
}

export interface Tasks {
  /** The root Taskfile's name, e.g. `Taskfile.yml`. */
  file: string
  tasks: TaskEntry[]
  refused: RefusedInclude[]
  /** The server stopped at its entry limit; later tasks and refusals are not listed. */
  truncated: boolean
}

/** A block's data: loading, ready, or a sentence saying why not. */
export type Load<T> = { state: 'loading' } | { state: 'ready'; data: T } | Unavailable

export interface Unavailable {
  state: 'unavailable'
  /** `absent`: the source does not exist here; `error`: it exists but could not be read. */
  reason: 'absent' | 'error'
  message: string
}

function record(value: unknown): Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

function list(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.map(record) : []
}

function text(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function textOrNull(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

function count(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function normaliseVcs(raw: unknown): Vcs {
  const vcs = record(raw)
  return {
    branch: textOrNull(vcs.branch),
    head: textOrNull(vcs.head),
    upstream: textOrNull(vcs.upstream),
    ahead: count(vcs.ahead),
    behind: count(vcs.behind),
    dirty: list(vcs.dirty).map((entry) => ({
      path: text(entry.path),
      status: text(entry.status),
      orig_path: textOrNull(entry.orig_path),
    })),
    commits: list(vcs.commits).map((entry) => ({
      sha: text(entry.sha),
      author: text(entry.author),
      date: text(entry.date),
      subject: text(entry.subject),
    })),
    tags: list(vcs.tags).map((entry) => ({
      name: text(entry.name),
      sha: text(entry.sha),
      date: text(entry.date),
    })),
    remotes: list(vcs.remotes).map((entry) => ({ name: text(entry.name), url: text(entry.url) })),
    worktrees: list(vcs.worktrees).map((entry) => ({
      path: text(entry.path),
      head: textOrNull(entry.head),
      branch: textOrNull(entry.branch),
      locked: entry.locked === true,
      prunable: entry.prunable === true,
    })),
    tags_error: textOrNull(vcs.tags_error),
    remotes_error: textOrNull(vcs.remotes_error),
    worktrees_error: textOrNull(vcs.worktrees_error),
  }
}

export function normaliseDocuments(raw: unknown): DocumentEntry[] {
  return list(raw)
    .map((entry) => ({ name: text(entry.name), present: entry.present === true }))
    .filter((entry) => entry.name !== '')
}

export function normaliseTasks(raw: unknown): Tasks {
  const tasks = record(raw)
  return {
    file: text(tasks.file),
    tasks: list(tasks.tasks)
      .map((entry) => ({
        name: text(entry.name),
        desc: textOrNull(entry.desc),
        summary: textOrNull(entry.summary),
        internal: entry.internal === true,
        aliases: Array.isArray(entry.aliases)
          ? entry.aliases.filter((alias): alias is string => typeof alias === 'string')
          : [],
      }))
      .filter((entry) => entry.name !== ''),
    refused: list(tasks.refused).map((entry) => ({
      include: text(entry.include),
      taskfile: textOrNull(entry.taskfile),
      reason: text(entry.reason),
    })),
    truncated: tasks.truncated === true,
  }
}

/** What a failed request means for a block. `messages` names the 404 and 503 cases. */
function unavailable<T>(
  result: Exclude<ApiResult<T>, { state: 'ready' }>,
  messages: { absent: string; unavailable: string; other?: Partial<Record<number, string>> },
): Unavailable {
  if (result.state === 'token-rejected') {
    return { state: 'unavailable', reason: 'error', message: TOKEN_REJECTED_MESSAGE }
  }
  if (result.status === 404)
    return { state: 'unavailable', reason: 'absent', message: messages.absent }
  if (result.status === 503) {
    return { state: 'unavailable', reason: 'error', message: messages.unavailable }
  }
  const known = result.status === null ? undefined : messages.other?.[result.status]
  return { state: 'unavailable', reason: 'error', message: known ?? result.message }
}

async function load<T>(
  path: string,
  normalise: (raw: unknown) => T,
  messages: Parameters<typeof unavailable>[1],
): Promise<Load<T>> {
  const result = await apiGet<unknown>(path)
  if (result.state === 'ready') return { state: 'ready', data: normalise(result.data) }
  return unavailable(result, messages)
}

export function loadVcs(): Promise<Load<Vcs>> {
  return load('vcs', normaliseVcs, {
    absent: 'not a Git work tree',
    unavailable: 'git could not be run here (not on PATH, or it failed)',
  })
}

export function loadDocuments(): Promise<Load<DocumentEntry[]>> {
  return load('docs', normaliseDocuments, {
    absent: 'no documents',
    unavailable: 'documents unavailable',
  })
}

export function loadDocument(name: string): Promise<Load<Document>> {
  return load(`docs/${name}`, (raw) => ({ name, markdown: text(record(raw).markdown) }), {
    absent: `${name} is absent`,
    unavailable: `${name} is unavailable`,
    other: { 413: `${name} is larger than 1 MiB and is not shown` },
  })
}

/** `/api/tasks`; a Taskfile the server could not read shows the server's diagnostic. */
export async function loadTasks(): Promise<Load<Tasks>> {
  const result = await apiGet<unknown>('tasks')
  if (result.state === 'ready') return { state: 'ready', data: normaliseTasks(result.data) }
  const diagnostic = result.state === 'error' ? textOrNull(record(result.body).diagnostic) : null
  return unavailable(result, {
    absent: 'Taskfile.yml absent',
    unavailable: diagnostic ?? 'the Taskfile could not be read',
  })
}

/** The first eight characters of a commit hash. */
export function shortSha(sha: string | null): string {
  return sha === null ? '' : sha.slice(0, 8)
}

/** `2026-10-05T14:03:11+02:00` as `2026-10-05 14:03`; anything else unchanged. */
export function shortDate(date: string): string {
  const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(date)
  return match ? `${match[1] ?? ''} ${match[2] ?? ''}` : date
}
