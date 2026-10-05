<script setup lang="ts">
// One artifact in the Plan tree, with its children below it when expanded.
import { inject, ref, watch } from 'vue'
import type { TreeNode } from '../../api/plan'
import { artifactRoute } from './links'
import { TREE_COMMAND } from './tree'

const props = defineProps<{ node: TreeNode; depth: number }>()

const command = inject(TREE_COMMAND, null)

// After "Expand all" or "Collapse all", a node that appears follows it; before either, visions
// and the level below them start open and deeper levels closed.
const expanded = ref(
  command !== null && command.value.serial > 0 ? command.value.open : props.depth < 2,
)

if (command !== null) {
  watch(
    () => command.value.serial,
    () => {
      expanded.value = command.value.open
    },
  )
}
</script>

<template>
  <li class="tree-node" data-test="tree-node" :data-id="node.artifact.id">
    <div class="tree-row">
      <button
        v-if="node.children.length > 0"
        type="button"
        class="tree-toggle"
        data-test="tree-toggle"
        :aria-expanded="expanded"
        :aria-label="`${expanded ? 'Collapse' : 'Expand'} ${node.artifact.id}`"
        @click="expanded = !expanded"
      >
        {{ expanded ? '▾' : '▸' }}
      </button>
      <span v-else class="tree-toggle-space" aria-hidden="true" />
      <span v-if="node.via.length > 0" class="tree-via" data-test="tree-via">{{
        node.via.join(', ')
      }}</span>
      <span class="plan-kind">{{ node.artifact.kind }}</span>
      <RouterLink :to="artifactRoute(node.artifact.id)" class="tree-link">
        {{ node.artifact.title }}
      </RouterLink>
      <code class="plan-id">{{ node.artifact.id }}</code>
      <span class="plan-status">{{ node.artifact.status }}</span>
      <span v-if="node.cycle" class="tree-cycle">cycle: shown above</span>
      <span v-else-if="!expanded && node.children.length > 0" class="tree-count">
        {{ node.children.length }} below
      </span>
    </div>
    <ul v-if="expanded && node.children.length > 0" class="tree-children" data-test="tree-children">
      <PlanTreeNode
        v-for="child in node.children"
        :key="child.artifact.id"
        :node="child"
        :depth="depth + 1"
      />
    </ul>
  </li>
</template>

<style scoped>
.tree-node {
  list-style: none;
}

.tree-row {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.45rem;
  padding: 0.2rem 0;
}

.tree-toggle {
  width: 1.4rem;
  padding: 0;
  border: none;
  background: none;
  color: var(--muted);
  font: inherit;
  cursor: pointer;
}

.tree-toggle-space {
  display: inline-block;
  width: 1.4rem;
}

.tree-via {
  font-size: 0.75rem;
  font-style: italic;
  color: var(--muted);
}

.tree-link {
  color: var(--fg);
  font-weight: 500;
}

.tree-cycle,
.tree-count {
  font-size: 0.75rem;
  color: var(--muted);
}

.tree-children {
  margin: 0 0 0 0.7rem;
  padding: 0 0 0 0.9rem;
  border-left: 1px solid var(--border);
}
</style>
