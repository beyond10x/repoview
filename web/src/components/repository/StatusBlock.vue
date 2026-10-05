<script setup lang="ts">
import { computed } from 'vue'
import { shortSha, type Vcs } from '../../api/repository'

const props = defineProps<{ vcs: Vcs }>()

const dirtyCount = computed(() =>
  props.vcs.dirty.length === 0 ? 'clean' : `${String(props.vcs.dirty.length)} changed`,
)
const hasCounts = computed(() => props.vcs.ahead !== null && props.vcs.behind !== null)
</script>

<template>
  <div class="status" data-test="vcs-status">
    <dl class="facts">
      <dt>branch</dt>
      <dd>
        <code v-if="vcs.branch !== null" data-test="branch">{{ vcs.branch }}</code>
        <span v-else class="detached" data-test="branch">detached HEAD</span>
      </dd>
      <dt>head</dt>
      <dd>
        <code v-if="vcs.head !== null" data-test="head" :title="vcs.head">{{
          shortSha(vcs.head)
        }}</code>
        <span v-else class="muted" data-test="head">no commits yet</span>
      </dd>
      <dt>upstream</dt>
      <dd data-test="upstream">
        <template v-if="vcs.upstream !== null">
          <code>{{ vcs.upstream }}</code>
          <template v-if="hasCounts">
            <span class="count" :class="{ zero: vcs.ahead === 0 }" data-test="ahead"
              >↑{{ vcs.ahead }}</span
            >
            <span class="count" :class="{ zero: vcs.behind === 0 }" data-test="behind"
              >↓{{ vcs.behind }}</span
            >
          </template>
          <span v-else class="muted"> ahead/behind unknown</span>
        </template>
        <span v-else class="muted">no upstream</span>
      </dd>
      <dt>work tree</dt>
      <dd>
        <span :class="vcs.dirty.length === 0 ? 'clean' : 'dirty'" data-test="dirty-count">{{
          dirtyCount
        }}</span>
      </dd>
    </dl>
    <table v-if="vcs.dirty.length > 0" class="dirty-files">
      <thead>
        <tr>
          <th scope="col" title="porcelain v2 XY: index, then work tree">XY</th>
          <th scope="col">path</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="file in vcs.dirty" :key="file.path" data-test="dirty-file">
          <td>
            <code class="letters" data-test="dirty-status">{{ file.status }}</code>
          </td>
          <td>
            <code data-test="dirty-path">{{
              file.orig_path === null ? file.path : `${file.orig_path} → ${file.path}`
            }}</code>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.facts {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.35rem 1rem;
  margin: 0 0 0.75rem;
}

.facts dt {
  color: var(--muted);
}

.facts dd {
  margin: 0;
}

.detached {
  color: var(--missing);
  font-weight: 600;
}

.muted {
  color: var(--muted);
}

.count {
  margin-left: 0.5rem;
  font-family: var(--mono);
  font-weight: 600;
}

.count.zero {
  color: var(--muted);
  font-weight: normal;
}

.clean {
  color: var(--present);
}

.dirty {
  color: var(--missing);
  font-weight: 600;
}

.dirty-files {
  border-collapse: collapse;
  font-size: 0.9rem;
}

.dirty-files th {
  text-align: left;
  color: var(--muted);
  font-weight: normal;
  padding-right: 1rem;
}

.dirty-files td {
  padding: 0.1rem 1rem 0.1rem 0;
  white-space: pre-wrap;
}

.letters {
  color: var(--missing);
}
</style>
