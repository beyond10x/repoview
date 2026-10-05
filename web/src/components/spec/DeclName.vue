<script setup lang="ts">
// A declaration's heading: its short name, then the qualified name it is known by.
import { computed } from 'vue'
import type { Naming } from '../../api/spec'
import { shortName } from './ir'

const props = defineProps<{ name: string; naming?: Naming | undefined }>()
const display = computed(() => props.naming?.display ?? shortName(props.name))
</script>

<template>
  <div class="decl-name">
    <strong class="short">{{ display }}</strong>
    <code class="qualified">{{ name }}</code>
  </div>
  <p v-if="naming?.summary" class="decl-summary">{{ naming.summary }}</p>
</template>

<style scoped>
.decl-name {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
}

.short {
  font-size: 1.05rem;
}

.qualified {
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.8rem;
}

.decl-summary {
  margin: 0.25rem 0 0;
  color: var(--muted);
}
</style>
