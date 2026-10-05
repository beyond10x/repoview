<script setup lang="ts">
// One ESS specification root: its validate result first, and, when ess accepts it, the compiled
// IR (domains, entities with lifecycles, commands, events, views, components) and the
// interaction graph.
import { computed, nextTick, shallowRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { ApiResult } from '../api/client'
import {
  loadGraph,
  loadIr,
  loadMermaid,
  loadRoots,
  rootFromParam,
  type Graph,
  type Ir,
  type RootEntry,
} from '../api/spec'
import EssProducer from '../components/spec/EssProducer.vue'
import IrSections from '../components/spec/IrSections.vue'
import RefusalList from '../components/spec/RefusalList.vue'

const route = useRoute()
const root = computed(() => rootFromParam(route.params.root ?? ''))

type Page =
  | { state: 'loading' }
  | { state: 'failed'; message: string }
  | { state: 'unknown' }
  | { state: 'refused'; entry: RootEntry }
  | { state: 'ir'; entry: RootEntry; ir: Ir }

const page = shallowRef<Page>({ state: 'loading' })
const graph = shallowRef<ApiResult<Graph> | null>(null)
const mermaid = shallowRef<ApiResult<string> | null>(null)

function failure(result: ApiResult<unknown>): string {
  return result.state === 'token-rejected'
    ? 'the server rejected the run token; reopen repoview from the URL it printed'
    : result.state === 'error'
      ? result.message
      : ''
}

let current = 0

async function load(name: string): Promise<void> {
  const run = ++current
  page.value = { state: 'loading' }
  graph.value = null
  mermaid.value = null
  const roots = await loadRoots()
  if (run !== current) return
  if (roots.state !== 'ready') {
    page.value = { state: 'failed', message: `specification roots unavailable: ${failure(roots)}` }
    return
  }
  const entry = roots.data.find((candidate) => candidate.root === name)
  if (entry === undefined) {
    page.value = { state: 'unknown' }
    return
  }
  if (!entry.ok) {
    page.value = { state: 'refused', entry }
    return
  }
  const [ir, loadedGraph, loadedMermaid] = await Promise.all([
    loadIr(name),
    loadGraph(name),
    loadMermaid(name),
  ])
  if (run !== current) return
  graph.value = loadedGraph
  mermaid.value = loadedMermaid
  page.value =
    ir.state === 'ready'
      ? { state: 'ir', entry, ir: ir.data }
      : { state: 'failed', message: `compiled IR unavailable: ${failure(ir)}` }
}

watch(root, (name) => void load(name), { immediate: true })

// The router has no scroll behaviour, so a `#entity-…` link (or one opened directly) is scrolled
// to here once its card exists.
watch(
  [() => route.hash, () => page.value.state],
  async ([hash]) => {
    if (hash === '') return
    await nextTick()
    const target = document.getElementById(decodeURIComponent(hash.slice(1)))
    if (target !== null && typeof target.scrollIntoView === 'function') target.scrollIntoView()
  },
  { immediate: true },
)

function reported(value: unknown): string | null {
  return typeof value === 'string' || typeof value === 'number' ? String(value) : null
}

const system = computed(() => {
  if (page.value.state === 'ir') return page.value.ir.system
  if (page.value.state === 'refused') return reported(page.value.entry.validate.system)
  return null
})
const version = computed(() =>
  page.value.state === 'ir' || page.value.state === 'refused'
    ? reported(page.value.entry.validate.version)
    : null,
)
</script>

<template>
  <section data-test="page" data-page="spec-root">
    <h1>Spec root</h1>
    <p class="heading" data-test="spec-root-heading">
      <strong v-if="system">{{ system }}</strong>
      <span v-if="version" class="version">{{ version }}</span>
      <code class="root">{{ root }}</code>
      <RouterLink to="/specs" class="back">all roots</RouterLink>
    </p>
    <EssProducer />
    <p v-if="page.state === 'loading'" class="notice">reading the specification…</p>
    <p v-else-if="page.state === 'failed'" class="notice notice-error" role="alert">
      {{ page.message }}
    </p>
    <p v-else-if="page.state === 'unknown'" class="notice notice-error" role="alert">
      <code>{{ root }}</code> is not a detected specification root
    </p>
    <div v-else-if="page.state === 'refused'" class="refused-root">
      <p class="notice notice-error" role="alert">
        ess refuses this specification, so it has no compiled IR to show
      </p>
      <RefusalList :validate="page.entry.validate" />
    </div>
    <IrSections v-else :ir="page.ir" :graph="graph" :mermaid="mermaid" />
  </section>
</template>

<style scoped>
.heading {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.75rem;
  margin: 0 0 0.25rem;
  font-size: 1.1rem;
}

.version {
  color: var(--muted);
}

.root {
  font-family: var(--mono);
  font-size: 0.9rem;
}

.back {
  margin-left: auto;
  color: var(--accent);
  font-size: 0.9rem;
}

.notice {
  margin-top: var(--gap);
}
</style>
