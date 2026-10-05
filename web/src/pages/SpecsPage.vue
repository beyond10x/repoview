<script setup lang="ts">
// Every ESS specification root the server detected, with what `ess specify validate` says.
import { onMounted, shallowRef } from 'vue'
import type { ApiResult } from '../api/client'
import { loadRoots, rootPagePath, type RootEntry } from '../api/spec'
import EssProducer from '../components/spec/EssProducer.vue'
import RefusalList from '../components/spec/RefusalList.vue'

const roots = shallowRef<ApiResult<RootEntry[]> | null>(null)

onMounted(async () => {
  roots.value = await loadRoots()
})

function reported(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.trim() !== ''
    ? value
    : typeof value === 'number'
      ? String(value)
      : fallback
}
</script>

<template>
  <section data-test="page" data-page="specs">
    <h1>Specs</h1>
    <EssProducer />
    <p v-if="roots === null" class="notice">reading specification roots…</p>
    <p v-else-if="roots.state === 'token-rejected'" class="notice notice-error" role="alert">
      the server rejected the run token; reopen repoview from the URL it printed
    </p>
    <p v-else-if="roots.state === 'error'" class="notice notice-error" role="alert">
      specification roots unavailable: {{ roots.message }}
    </p>
    <p v-else-if="roots.data.length === 0" class="notice" data-test="spec-empty">
      no ESS specification root detected (no ess-inputs.yaml or system.yaml outside ignored paths)
    </p>
    <table v-else class="roots">
      <thead>
        <tr>
          <th>root</th>
          <th>system</th>
          <th>version</th>
          <th>format</th>
          <th>validate</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="entry in roots.data"
          :key="entry.root"
          data-test="spec-root-row"
          :data-root="entry.root"
          :data-ok="String(entry.ok)"
        >
          <td>
            <RouterLink :to="rootPagePath(entry.root)">
              <code>{{ entry.root }}</code>
            </RouterLink>
          </td>
          <td data-test="system">{{ reported(entry.validate.system, 'not reported') }}</td>
          <td data-test="version">{{ reported(entry.validate.version, 'not reported') }}</td>
          <td data-test="format">{{ reported(entry.validate.format, 'not reported by ess') }}</td>
          <td data-test="validate">
            <span v-if="entry.ok" class="valid">valid</span>
            <RefusalList v-else :validate="entry.validate" />
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.roots {
  margin-top: var(--gap);
  border-collapse: collapse;
  width: 100%;
}

th {
  color: var(--muted);
  font-weight: 400;
  text-align: left;
}

th,
td {
  padding: 0.4rem 1rem 0.4rem 0;
  border-bottom: 1px solid var(--border);
  vertical-align: top;
}

td a {
  color: var(--accent);
}

.valid {
  color: var(--present);
  font-weight: 600;
}

.notice {
  margin-top: var(--gap);
}
</style>
