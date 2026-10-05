<script setup lang="ts">
// Which `ess` answered, from the snapshot's `spec` source: every panel names its producer.
import { computed } from 'vue'
import { useSnapshot } from '../../composables/useSnapshot'
import { orElse } from '../../display'

const snapshot = useSnapshot()
const line = computed(() => {
  if (snapshot.value.state !== 'ready') return 'producer: reading the snapshot…'
  const spec = snapshot.value.snapshot.sources.find((source) => source.source_id === 'spec')
  if (spec === undefined) return 'producer: the snapshot has no spec source'
  if (spec.availability === 'Present') {
    const version = orElse(spec.tool_version, 'ess, version not reported')
    return `answered by ${version} · ${orElse(spec.tool_path, 'path unknown')}`
  }
  return `ess: ${spec.availability} · ${orElse(spec.diagnostic, 'no diagnostic')}`
})
</script>

<template>
  <p class="producer" data-test="producer">{{ line }}</p>
</template>

<style scoped>
.producer {
  margin: 0;
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.8rem;
}
</style>
