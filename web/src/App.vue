<script setup lang="ts">
import { computed } from 'vue'
import { useSnapshot } from './composables/useSnapshot'
import { orElse } from './display'
import { NAV, type NavEntry } from './pages/nav'

const snapshot = useSnapshot()
const project = computed(() =>
  snapshot.value.state === 'ready' ? snapshot.value.snapshot.project : null,
)

// Dimmed only when the snapshot says every source the page reads is Absent; never hidden.
function isAbsent(entry: NavEntry): boolean {
  if (snapshot.value.state !== 'ready' || entry.sources.length === 0) return false
  const sources = snapshot.value.snapshot.sources.filter((source) =>
    entry.sources.includes(source.source_id),
  )
  return sources.length > 0 && sources.every((source) => source.availability === 'Absent')
}
</script>

<template>
  <header class="top-bar" data-test="top-bar">
    <span class="brand">repoview</span>
    <template v-if="project">
      <strong class="project-name">{{ orElse(project.name, 'unnamed project') }}</strong>
      <code class="project-root">{{ orElse(project.root, 'unknown root') }}</code>
    </template>
  </header>
  <div class="layout">
    <nav class="nav" data-test="nav">
      <RouterLink
        v-for="entry in NAV"
        :key="entry.to"
        :to="entry.to"
        :data-nav="entry.title"
        :data-absent="String(isAbsent(entry))"
        :class="{ 'nav-absent': isAbsent(entry) }"
      >
        {{ entry.title }}<span v-if="isAbsent(entry)" class="nav-absent-label"> absent</span>
      </RouterLink>
    </nav>
    <main class="main">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.nav-absent {
  opacity: 0.55;
}

.nav-absent-label {
  color: var(--absent);
  font-size: 0.8em;
}
</style>
