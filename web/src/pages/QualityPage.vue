<script setup lang="ts">
// /api/quality, read once: no Codegate release assesses source yet, so nothing runs to poll for.
import { onBeforeUnmount, onMounted, shallowRef } from 'vue'
import { CODEGATE_REPOSITORY, loadQuality, type QualityState } from '../api/quality'
import CodegateReport from '../components/quality/CodegateReport.vue'

const quality = shallowRef<QualityState>({ state: 'loading' })
let left = false

onMounted(() => {
  void loadQuality().then((next) => {
    if (!left) quality.value = next
  })
})

onBeforeUnmount(() => {
  left = true
})
</script>

<template>
  <section class="quality" data-test="page" data-page="quality">
    <h1>Quality</h1>
    <CodegateReport v-if="quality.state === 'ready'" :report="quality.report" />
    <p v-else-if="quality.state === 'loading'" class="notice">loading quality…</p>
    <div v-else-if="quality.state === 'unavailable'" class="notice" data-test="unavailable">
      <p>{{ quality.message }}</p>
      <p>
        <a :href="CODEGATE_REPOSITORY" data-test="codegate-link" rel="noopener noreferrer">
          {{ CODEGATE_REPOSITORY }}
        </a>
      </p>
    </div>
    <p v-else class="notice notice-error" role="alert">{{ quality.message }}</p>
  </section>
</template>
