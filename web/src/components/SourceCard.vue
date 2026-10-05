<script setup lang="ts">
import { computed } from 'vue'
import type { Source } from '../api/snapshot'
import { hasText, orElse } from '../display'

// `to`: the page this source belongs to; without it the card has no link.
const props = defineProps<{ source: Source; to?: string | null }>()

const status = computed(() => {
  const { availability, tool } = props.source
  switch (availability) {
    case 'Present':
      return 'present'
    case 'Absent':
      return 'absent'
    case 'ToolMissing':
      return `tool missing: ${orElse(tool, 'unknown tool')}`
    case 'Failed':
      return 'failed'
  }
  return `unknown availability: ${String(availability)}`
})

// The server names the tool on every source that has one, whether or not it ran. Only Present and
// Failed mean the tool answered, so only they show the tool line and path. ToolMissing names the
// tool in its status; Absent shows no tool at all.
const toolRan = computed(
  () => props.source.availability === 'Present' || props.source.availability === 'Failed',
)

const toolLine = computed<string | null>(() => {
  const { tool, tool_version } = props.source
  if (!toolRan.value) return null
  if (!hasText(tool) && !hasText(tool_version)) return 'no external tool'
  return `${orElse(tool, 'unknown tool')} · ${orElse(tool_version, 'version unknown')}`
})

const toolPath = computed(() =>
  toolRan.value && hasText(props.source.tool_path) ? props.source.tool_path : null,
)

const diagnostic = computed<string | null>(() => {
  const { availability, diagnostic } = props.source
  if (availability === 'Failed') return orElse(diagnostic, 'the tool gave no diagnostic')
  return hasText(diagnostic) ? diagnostic : null
})

function describe(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  if (Array.isArray(value)) {
    return value
      .map(describe)
      .filter((part) => hasText(part))
      .join(', ')
  }
  return JSON.stringify(value)
}

const summaryRows = computed<Array<[string, string]>>(() => {
  const summary = props.source.summary
  if (props.source.kind === 'Vcs') {
    const rows: Array<[string, string]> = []
    // A key the server sent keeps its row even when its value is null or empty.
    if ('branch' in summary) {
      const branch = typeof summary.branch === 'string' ? summary.branch : null
      rows.push(['branch', orElse(branch, 'detached HEAD')])
    }
    if ('head' in summary) {
      const head = typeof summary.head === 'string' ? summary.head.slice(0, 7) : null
      rows.push(['head', orElse(head, 'no commit')])
    }
    if ('dirty' in summary) {
      const dirty = summary.dirty
      const worktree =
        typeof dirty !== 'number' ? 'unknown' : dirty === 0 ? 'clean' : `${String(dirty)} changed`
      rows.push(['worktree', worktree])
    }
    return rows
  }
  return Object.entries(summary).map(([key, value]) => [
    orElse(key, 'unnamed'),
    orElse(describe(value), 'none'),
  ])
})
</script>

<template>
  <article
    class="card"
    data-test="source-card"
    :data-source="source.source_id"
    :data-availability="source.availability"
  >
    <header class="card-header">
      <h2>
        <RouterLink v-if="to" :to="to">{{ orElse(source.source_id, 'unnamed source') }}</RouterLink>
        <template v-else>{{ orElse(source.source_id, 'unnamed source') }}</template>
      </h2>
      <span class="kind">{{ source.kind }}</span>
    </header>
    <p class="location">{{ orElse(source.location, 'no location') }}</p>
    <p class="status" :class="`status-${source.availability}`" data-test="source-status">
      {{ status }}
    </p>
    <p v-if="toolLine !== null" class="tool">{{ toolLine }}</p>
    <p v-if="toolPath !== null" class="tool-path">{{ toolPath }}</p>
    <pre v-if="diagnostic !== null" class="diagnostic">{{ diagnostic }}</pre>
    <dl v-if="summaryRows.length > 0" class="summary">
      <template v-for="[key, value] in summaryRows" :key="key">
        <dt>{{ key }}</dt>
        <dd>{{ value }}</dd>
      </template>
    </dl>
  </article>
</template>
