<script setup lang="ts">
import { TOKEN_REJECTED_MESSAGE } from '../api/snapshot'
import SourceCard from '../components/SourceCard.vue'
import { useSnapshot } from '../composables/useSnapshot'

const snapshot = useSnapshot()
</script>

<template>
  <section class="overview">
    <h1>Overview</h1>
    <p v-if="snapshot.state === 'loading'" class="notice">loading snapshot…</p>
    <p v-else-if="snapshot.state === 'token-rejected'" class="notice notice-error" role="alert">
      {{ TOKEN_REJECTED_MESSAGE }}
    </p>
    <p v-else-if="snapshot.state === 'error'" class="notice notice-error" role="alert">
      {{ snapshot.message }}
    </p>
    <div v-else class="cards">
      <SourceCard
        v-for="source in snapshot.snapshot.sources"
        :key="source.source_id"
        :source="source"
      />
    </div>
  </section>
</template>
