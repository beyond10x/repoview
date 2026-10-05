<script setup lang="ts">
// The Taskfile's tasks as written: names, aliases, descriptions and summaries are text.
import type { Tasks } from '../../api/repository'

defineProps<{ tasks: Tasks }>()
</script>

<template>
  <p class="muted source">
    read from <code data-test="tasks-file">{{ tasks.file }}</code>
  </p>
  <p v-if="tasks.tasks.length === 0" class="empty" data-test="empty">no tasks</p>
  <table v-else class="rows">
    <tbody>
      <tr v-for="task in tasks.tasks" :key="task.name" data-test="task">
        <td class="nowrap">
          <code data-test="task-name">{{ task.name }}</code>
          <span v-if="task.internal" class="flag" data-test="task-internal">internal</span>
          <div v-if="task.aliases.length > 0" class="muted mono" data-test="task-aliases">
            {{ task.aliases.join(', ') }}
          </div>
        </td>
        <td>
          <span data-test="task-desc">{{ task.desc ?? '' }}</span>
          <details v-if="task.summary !== null">
            <summary class="muted">summary</summary>
            <pre class="summary" data-test="task-summary">{{ task.summary }}</pre>
          </details>
        </td>
      </tr>
    </tbody>
  </table>
  <p
    v-for="entry in tasks.refused"
    :key="entry.include"
    class="block-error"
    data-test="task-refused"
  >
    include <code>{{ entry.include }}</code
    ><template v-if="entry.taskfile !== null">
      (<code>{{ entry.taskfile }}</code
      >)</template
    >
    not read: {{ entry.reason }}
  </p>
  <p v-if="tasks.truncated" class="block-error" data-test="tasks-truncated">
    list truncated at 10,000 entries
  </p>
</template>

<style scoped src="./tables.css"></style>
<style scoped>
.source {
  margin: 0 0 0.5rem;
  font-size: 0.85rem;
}

.summary {
  margin: 0.25rem 0 0;
  white-space: pre-wrap;
  font-family: var(--mono);
  font-size: 0.8rem;
}

.block-error {
  margin-top: 0.5rem;
}
</style>
