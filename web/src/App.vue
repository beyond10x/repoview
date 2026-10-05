<script setup lang="ts">
import { computed } from 'vue'
import { useSnapshot } from './composables/useSnapshot'
import { orElse } from './display'

const snapshot = useSnapshot()
const project = computed(() =>
  snapshot.value.state === 'ready' ? snapshot.value.snapshot.project : null,
)
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
      <RouterLink to="/">Overview</RouterLink>
    </nav>
    <main class="main">
      <RouterView />
    </main>
  </div>
</template>
