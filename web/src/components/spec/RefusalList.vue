<script setup lang="ts">
// The lines a validate result refuses a root with, each verbatim.
import { computed } from 'vue'
import { refusalLines } from '../../api/spec'

const props = defineProps<{ validate: Record<string, unknown> }>()
const lines = computed(() => refusalLines(props.validate))
</script>

<template>
  <div class="refusal" data-test="refusal">
    <span class="refused">refused</span>
    <ul>
      <li v-for="(line, index) in lines" :key="index" data-test="refusal-line">{{ line }}</li>
    </ul>
  </div>
</template>

<style scoped>
.refused {
  color: var(--failed);
  font-weight: 600;
}

ul {
  margin: 0.25rem 0 0;
  padding-left: 0;
  list-style: none;
}

li {
  padding: 0.25rem 0.5rem;
  border-left: 3px solid var(--failed);
  font-family: var(--mono);
  font-size: 0.8rem;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

li + li {
  margin-top: 0.25rem;
}
</style>
