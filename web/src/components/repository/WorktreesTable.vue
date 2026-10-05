<script setup lang="ts">
import { shortSha, type Worktree } from '../../api/repository'

defineProps<{ worktrees: Worktree[]; error?: string | null }>()
</script>

<template>
  <p v-if="error" class="block-error" data-test="block-error">{{ error }}</p>
  <p v-else-if="worktrees.length === 0" class="empty" data-test="empty">no worktrees</p>
  <table v-else class="rows">
    <thead>
      <tr>
        <th scope="col">path</th>
        <th scope="col">branch</th>
        <th scope="col">head</th>
        <th scope="col"><span class="visually-hidden">state</span></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="worktree in worktrees" :key="worktree.path" data-test="worktree">
        <td class="mono" data-test="worktree-path">{{ worktree.path }}</td>
        <td>
          <code v-if="worktree.branch !== null" data-test="worktree-branch">{{
            worktree.branch
          }}</code>
          <span v-else class="muted" data-test="worktree-branch">detached</span>
        </td>
        <td class="mono">
          <code v-if="worktree.head !== null" data-test="worktree-head" :title="worktree.head">{{
            shortSha(worktree.head)
          }}</code>
          <span v-else class="muted" data-test="worktree-head">no commits yet</span>
        </td>
        <td data-test="worktree-flags">
          <span v-if="worktree.locked" class="flag">locked</span>
          <span v-if="worktree.prunable" class="flag">prunable</span>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped src="./tables.css"></style>

<style scoped>
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}
</style>
