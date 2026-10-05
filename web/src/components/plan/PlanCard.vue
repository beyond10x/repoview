<script setup lang="ts">
import type { PlanItem } from '../../api/plan'
import { artifactRoute } from './links'

defineProps<{ artifact: PlanItem }>()
</script>

<template>
  <li class="plan-card" data-test="plan-card" :data-id="artifact.id">
    <RouterLink :to="artifactRoute(artifact.id)" class="plan-card-link">
      <span class="plan-card-meta">
        <span class="plan-kind" data-test="card-kind">{{ artifact.kind }}</span>
        <code class="plan-id" data-test="card-id">{{ artifact.id }}</code>
      </span>
      <span class="plan-card-title" data-test="card-title">{{ artifact.title }}</span>
      <span v-if="(artifact.blocked_by ?? []).length > 0" class="plan-card-blocked">
        blocked by {{ (artifact.blocked_by ?? []).map((entry) => entry.blocker).join(', ') }}
      </span>
      <span v-if="(artifact.tags ?? []).length > 0" class="plan-card-tags">
        <span v-for="tag in artifact.tags" :key="tag" class="plan-tag">{{ tag }}</span>
      </span>
    </RouterLink>
  </li>
</template>

<style scoped>
.plan-card {
  list-style: none;
}

.plan-card-link {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.6rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--fg);
  text-decoration: none;
}

.plan-card-link:hover,
.plan-card-link:focus-visible {
  border-color: var(--accent);
}

.plan-card-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.4rem;
}

.plan-card-title {
  font-weight: 500;
  line-height: 1.3;
}

.plan-card-blocked {
  font-size: 0.8rem;
  color: var(--missing);
}

.plan-card-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}
</style>
