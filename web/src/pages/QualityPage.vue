<script setup lang="ts">
import { onBeforeUnmount, onMounted, shallowRef } from 'vue'
import {
  POLL_INTERVAL_MS,
  isRunning,
  loadQuality,
  type QualityReport,
  type QualityState,
} from '../api/quality'
import LanguageCard from '../components/quality/LanguageCard.vue'

// The last report the server gave; a failed poll keeps it on screen and says so.
const report = shallowRef<QualityReport | null>(null)
// Why there is no report: set only while `report` is null.
const problem = shallowRef<Exclude<QualityState, { state: 'ready' }>>({ state: 'loading' })
const pollError = shallowRef<string | null>(null)

let timer: ReturnType<typeof setTimeout> | undefined
let left = false

async function refresh(): Promise<void> {
  const next = await loadQuality()
  if (left) return
  if (next.state === 'ready') {
    report.value = next.report
    pollError.value = null
  } else if (report.value === null) {
    problem.value = next
  } else if (next.state !== 'loading') {
    pollError.value = next.message
  }
  if (report.value !== null && isRunning(report.value)) {
    timer = setTimeout(() => void refresh(), POLL_INTERVAL_MS)
  }
}

onMounted(() => void refresh())

onBeforeUnmount(() => {
  left = true
  clearTimeout(timer)
})
</script>

<template>
  <section class="quality" data-test="page" data-page="quality">
    <h1>Quality</h1>
    <template v-if="report !== null">
      <p class="producer" data-test="producer">
        assessed by <strong>{{ report.tool }}</strong>
        <code>{{ report.tool_path }}</code>
      </p>
      <p v-if="pollError !== null" class="notice notice-error" data-test="poll-error">
        refresh failed, showing the last answer: {{ pollError }}
      </p>
      <p v-if="report.languages.length === 0" class="notice" data-test="no-languages">
        no language detected at the project root, so there is nothing to assess
      </p>
      <div v-else class="language-cards">
        <LanguageCard v-for="entry in report.languages" :key="entry.language" :entry="entry" />
      </div>
    </template>
    <p v-else-if="problem.state === 'loading'" class="notice">loading quality…</p>
    <p v-else-if="problem.state === 'unavailable'" class="notice" data-test="unavailable">
      {{ problem.message }}
    </p>
    <p v-else class="notice notice-error" role="alert">{{ problem.message }}</p>
  </section>
</template>

<style scoped>
.producer {
  margin: 0 0 var(--gap);
  color: var(--muted);
}

.producer code {
  font-family: var(--mono);
  margin-left: 0.4rem;
}

.language-cards {
  display: grid;
  gap: var(--gap);
  max-width: 60rem;
}
</style>
