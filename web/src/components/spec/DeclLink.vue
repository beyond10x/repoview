<script setup lang="ts">
// A link to another declaration's card on the same page, or its plain name when the page has no
// card for it. The link keeps the current path as it is, so a root's `/` is never re-encoded.
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { shortName } from './ir'

const props = defineProps<{ name: string; anchor: string | null }>()
const route = useRoute()
const to = computed(() =>
  props.anchor === null ? null : { path: route.path, hash: `#${props.anchor}` },
)
</script>

<template>
  <RouterLink v-if="to" :to="to" :title="name" class="decl-link">{{ shortName(name) }}</RouterLink>
  <code v-else :title="name" class="decl-plain">{{ shortName(name) }}</code>
</template>

<style scoped>
.decl-link,
.decl-plain {
  font-family: var(--mono);
  font-size: 0.9em;
}

.decl-link {
  color: var(--accent);
}
</style>
