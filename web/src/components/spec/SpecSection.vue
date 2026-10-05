<script setup lang="ts">
// One titled section of a spec root page; says "none declared" instead of rendering empty.
defineProps<{ id: string; title: string; count: number }>()
</script>

<template>
  <section :id="`section-${id}`" class="spec-section" data-test="spec-section" :data-section="id">
    <h2 :data-count="count">{{ title }}</h2>
    <p v-if="count === 0" class="none">none declared</p>
    <slot v-else />
  </section>
</template>

<style scoped>
.spec-section {
  margin-top: 2rem;
}

h2 {
  margin: 0 0 0.75rem;
  padding-bottom: 0.25rem;
  border-bottom: 1px solid var(--border);
  font-size: 1.2rem;
}

/* The count is decoration: kept out of the heading text. */
h2::after {
  content: attr(data-count);
  margin-left: 0.5rem;
  color: var(--muted);
  font-size: 0.85rem;
  font-weight: 400;
}

.none {
  color: var(--muted);
}
</style>
