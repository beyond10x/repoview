<script setup lang="ts">
// A failed `/api/plan/*` answer: aep's own stderr when the server passed it on, else the reason.
import { computed } from 'vue'
import type { ApiResult } from '../../api/client'
import { failureText, toolFailure } from '../../api/plan'

const props = defineProps<{ result: ApiResult<unknown> }>()

const failure = computed(() => toolFailure(props.result))

const headline = computed(() => {
  const found = failure.value
  if (found === null) return 'could not load'
  if (found.exit === null) return `${found.tool} did not run to completion`
  return `${found.tool} exited with status ${String(found.exit)}`
})
</script>

<template>
  <div class="tool-failure" data-test="tool-failure" role="alert">
    <p class="notice-error">{{ headline }}</p>
    <pre class="diagnostic">{{ failureText(result) }}</pre>
  </div>
</template>

<style scoped>
.tool-failure p {
  margin: 0 0 0.25rem;
}
</style>
