<script setup lang="ts">
// The plan from vision down: serves, designs, decomposes and implements, from `plan/artifacts`.
import { computed, provide, ref, shallowRef } from 'vue'
import type { ApiResult } from '../api/client'
import { buildTree, loadArtifacts, type PlanItem } from '../api/plan'
import PlanTreeNode from '../components/plan/PlanTreeNode.vue'
import ToolFailureView from '../components/plan/ToolFailureView.vue'
import { TREE_COMMAND, type TreeCommand } from '../components/plan/tree'
import '../components/plan/plan.css'

const result = shallowRef<ApiResult<PlanItem[]> | null>(null)

void loadArtifacts().then((loaded) => {
  result.value = loaded
})

const tree = computed(() => (result.value?.state === 'ready' ? buildTree(result.value.data) : null))

const command = ref<TreeCommand>({ serial: 0, open: true })
provide(TREE_COMMAND, command)

function setAll(open: boolean): void {
  command.value = { serial: command.value.serial + 1, open }
}
</script>

<template>
  <section data-test="page" data-page="plan-tree">
    <header class="plan-header">
      <h1>Plan · Tree</h1>
      <nav class="plan-views">
        <RouterLink to="/plan">Board</RouterLink>
        <RouterLink to="/plan/tree">Tree</RouterLink>
      </nav>
    </header>

    <p v-if="result === null" class="plan-loading">Loading the plan…</p>
    <ToolFailureView v-else-if="result.state !== 'ready'" :result="result" />
    <template v-else-if="tree !== null">
      <div class="tree-tools">
        <button type="button" data-test="expand-all" @click="setAll(true)">Expand all</button>
        <button type="button" data-test="collapse-all" @click="setAll(false)">Collapse all</button>
      </div>
      <p v-if="tree.roots.length === 0" class="plan-loading">The plan has no vision.</p>
      <ul class="tree-group" data-test="tree-roots">
        <PlanTreeNode v-for="root in tree.roots" :key="root.artifact.id" :node="root" :depth="0" />
      </ul>
      <template v-if="tree.unattached.length > 0">
        <h2 data-test="tree-unattached-heading">
          Unattached <span class="tree-unattached-count">{{ tree.unattached.length }}</span>
        </h2>
        <p class="tree-unattached-note">
          Not below any vision through serves, designs, decomposes or implements.
        </p>
        <ul class="tree-group" data-test="tree-unattached">
          <PlanTreeNode
            v-for="node in tree.unattached"
            :key="node.artifact.id"
            :node="node"
            :depth="0"
          />
        </ul>
      </template>
    </template>
  </section>
</template>

<style scoped>
.tree-tools {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.tree-tools button {
  padding: 0.2rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  color: var(--fg);
  font: inherit;
  cursor: pointer;
}

.tree-group {
  margin: 0;
  padding: 0;
}

h2 {
  margin: 1.5rem 0 0.25rem;
  font-size: 1.1rem;
}

.tree-unattached-count {
  color: var(--muted);
  font-weight: normal;
}

.tree-unattached-note {
  margin: 0 0 0.5rem;
  color: var(--muted);
  font-size: 0.85rem;
}
</style>
