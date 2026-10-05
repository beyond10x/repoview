<script setup lang="ts">
// A tab per present document, each rendered through MarkdownView (the only way repository markdown
// reaches the DOM); absent documents are listed as such, never hidden.
import { computed, ref, watch } from 'vue'
import { loadDocument, type Document, type DocumentEntry, type Load } from '../../api/repository'
import MarkdownView from '../MarkdownView.vue'

const props = defineProps<{ documents: DocumentEntry[] }>()

const present = computed(() => props.documents.filter((entry) => entry.present))
const absent = computed(() => props.documents.filter((entry) => !entry.present))

const selected = ref<string | null>(null)
const loaded = ref<Record<string, Load<Document>>>({})

function select(name: string): void {
  selected.value = name
  if (loaded.value[name] !== undefined) return
  loaded.value = { ...loaded.value, [name]: { state: 'loading' } }
  void loadDocument(name).then((result) => {
    loaded.value = { ...loaded.value, [name]: result }
  })
}

watch(
  present,
  (entries) => {
    const first = entries[0]
    if (first === undefined) {
      selected.value = null
    } else if (!entries.some((entry) => entry.name === selected.value)) {
      select(first.name)
    }
  },
  { immediate: true },
)

const current = computed<Load<Document> | null>(() =>
  selected.value === null ? null : (loaded.value[selected.value] ?? null),
)

function tabId(name: string): string {
  return `doc-tab-${name.replace(/[^A-Za-z0-9]/g, '-')}`
}
</script>

<template>
  <div class="documents">
    <div class="tab-row">
      <div v-if="present.length > 0" class="tabs" role="tablist" aria-label="Documents">
        <button
          v-for="entry in present"
          :id="tabId(entry.name)"
          :key="entry.name"
          type="button"
          role="tab"
          class="tab"
          :aria-selected="entry.name === selected"
          data-test="doc-tab"
          :data-doc="entry.name"
          @click="select(entry.name)"
        >
          {{ entry.name }}
        </button>
      </div>
      <ul v-if="absent.length > 0" class="absent">
        <li v-for="entry in absent" :key="entry.name" data-test="doc-absent">
          {{ entry.name }} <span class="absent-label">absent</span>
        </li>
      </ul>
    </div>
    <div
      v-if="selected !== null"
      class="panel"
      role="tabpanel"
      :aria-labelledby="tabId(selected)"
      data-test="doc-panel"
    >
      <p v-if="current === null || current.state === 'loading'" class="muted">loading…</p>
      <p v-else-if="current.state === 'unavailable'" class="notice-error">{{ current.message }}</p>
      <MarkdownView v-else :source="current.data.markdown" />
    </div>
  </div>
</template>

<style scoped>
.tab-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem 1rem;
  border-bottom: 1px solid var(--border);
}

.tabs {
  display: flex;
  gap: 0.25rem;
}

.tab {
  padding: 0.4rem 0.8rem;
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: var(--radius) var(--radius) 0 0;
  background: none;
  color: var(--muted);
  font: inherit;
  font-family: var(--mono);
  cursor: pointer;
}

.tab[aria-selected='true'] {
  border-color: var(--border);
  background: var(--bg);
  color: var(--fg);
  font-weight: 600;
  margin-bottom: -1px;
}

.absent {
  display: flex;
  gap: 0.75rem;
  margin: 0;
  padding: 0;
  list-style: none;
  font-family: var(--mono);
  font-size: 0.85rem;
  color: var(--absent);
}

.absent-label {
  font-family: inherit;
  opacity: 0.8;
}

.panel {
  padding: 0.5rem 0.25rem;
  max-height: 70vh;
  overflow: auto;
}

.muted {
  color: var(--muted);
}
</style>
