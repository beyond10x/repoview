<script setup lang="ts">
// One artifact: `show` (with the body rendered), its relations both ways, `history` and `explain`.
import { computed, shallowRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { ApiResult } from '../api/client'
import {
  incomingRelations,
  loadArtifact,
  loadArtifacts,
  loadExplain,
  loadHistory,
  oldestFirst,
  stepDetails,
  evidenceText,
  historyText,
  type Artifact,
  type Explain,
  type HistoryEntry,
  type PlanItem,
} from '../api/plan'
import MarkdownView from '../components/MarkdownView.vue'
import { artifactRoute } from '../components/plan/links'
import ToolFailureView from '../components/plan/ToolFailureView.vue'
import '../components/plan/plan.css'

const route = useRoute()
const id = computed(() => {
  const param = route.params.id
  return Array.isArray(param) ? (param[0] ?? '') : (param ?? '')
})

const show = shallowRef<ApiResult<Artifact> | null>(null)
const history = shallowRef<ApiResult<HistoryEntry[]> | null>(null)
const explain = shallowRef<ApiResult<Explain> | null>(null)
const list = shallowRef<ApiResult<PlanItem[]> | null>(null)

// A later navigation wins: an answer for an id the page has left is dropped.
let current = 0
watch(
  id,
  (next) => {
    const load = ++current
    show.value = null
    history.value = null
    explain.value = null
    void loadArtifact(next).then((loaded) => {
      if (load === current) show.value = loaded
    })
    void loadHistory(next).then((loaded) => {
      if (load === current) history.value = loaded
    })
    void loadExplain(next).then((loaded) => {
      if (load === current) explain.value = loaded
    })
    // The whole list only feeds "Pointing here"; one read serves every artifact visited.
    if (list.value?.state !== 'ready') {
      void loadArtifacts().then((loaded) => {
        list.value = loaded
      })
    }
  },
  { immediate: true },
)

const artifact = computed(() => (show.value?.state === 'ready' ? show.value.data : null))
const incoming = computed(() =>
  list.value?.state === 'ready' ? incomingRelations(list.value.data, id.value) : [],
)
const entries = computed(() =>
  history.value?.state === 'ready' ? oldestFirst(history.value.data) : [],
)
const explained = computed(() => (explain.value?.state === 'ready' ? explain.value.data : null))

function location(file: string | null | undefined, line: number | null | undefined): string {
  if (file === null || file === undefined || file === '') return ''
  return line === null || line === undefined ? file : `${file}:${String(line)}`
}
</script>

<template>
  <section data-test="page" data-page="plan-artifact">
    <header class="plan-header">
      <h1>Plan · Artifact</h1>
      <nav class="plan-views">
        <RouterLink to="/plan">Board</RouterLink>
        <RouterLink to="/plan/tree">Tree</RouterLink>
      </nav>
    </header>

    <p v-if="show === null" class="plan-loading">Loading {{ id }}…</p>
    <template v-else-if="show.state !== 'ready'">
      <h2 class="artifact-title" data-test="artifact-title">
        <code>{{ id }}</code>
      </h2>
      <ToolFailureView :result="show" />
    </template>
    <article v-else-if="artifact !== null" class="artifact">
      <header class="artifact-header">
        <div class="artifact-meta">
          <span class="plan-kind" data-test="artifact-kind">{{ artifact.kind }}</span>
          <code class="plan-id">{{ artifact.id }}</code>
          <span class="plan-status" data-test="artifact-status">{{ artifact.status }}</span>
          <span
            v-if="artifact.revision !== undefined"
            class="artifact-revision"
            data-test="artifact-revision"
            >revision {{ artifact.revision }}</span
          >
          <span v-if="artifact.owner" class="artifact-owner">owner {{ artifact.owner }}</span>
          <span v-for="tag in artifact.tags ?? []" :key="tag" class="plan-tag">{{ tag }}</span>
        </div>
        <h2 class="artifact-title" data-test="artifact-title">{{ artifact.title }}</h2>
        <p v-if="artifact.summary" class="artifact-summary" data-test="artifact-summary">
          {{ artifact.summary }}
        </p>
      </header>

      <div class="artifact-layout">
        <div class="artifact-body" data-test="artifact-body">
          <MarkdownView v-if="artifact.body" :source="artifact.body" />
          <p v-else class="plan-loading">This artifact has no body.</p>
        </div>

        <aside class="artifact-side">
          <section class="side-section">
            <h2>Relations</h2>
            <ul v-if="artifact.relations.length > 0" class="relations" data-test="relations-out">
              <li v-for="(relation, index) in artifact.relations" :key="index">
                <span class="relation-name" data-test="relation-name">{{ relation.relation }}</span>
                <RouterLink :to="artifactRoute(relation.target)">{{ relation.target }}</RouterLink>
              </li>
            </ul>
            <p v-else class="plan-loading">none</p>
            <h3>Pointing here</h3>
            <ul v-if="incoming.length > 0" class="relations" data-test="relations-in">
              <li v-for="(relation, index) in incoming" :key="index">
                <RouterLink :to="artifactRoute(relation.source)">{{ relation.source }}</RouterLink>
                <span class="relation-name" data-test="relation-name">{{ relation.relation }}</span>
              </li>
            </ul>
            <p v-else-if="list?.state === 'ready'" class="plan-loading">none</p>
            <ToolFailureView v-else-if="list !== null" :result="list" />
          </section>

          <section v-if="(artifact.scope ?? []).length > 0" class="side-section">
            <h2>Scope</h2>
            <ul class="scope" data-test="scope">
              <li v-for="entry in artifact.scope" :key="entry.path">
                <code>{{ entry.path }}</code>
                <span v-if="entry.confidence" class="scope-confidence">{{ entry.confidence }}</span>
              </li>
            </ul>
          </section>

          <section class="side-section" data-test="explain">
            <h2>Why this status</h2>
            <p v-if="explain === null" class="plan-loading">Loading…</p>
            <ToolFailureView v-else-if="explain.state !== 'ready'" :result="explain" />
            <template v-else-if="explained !== null">
              <p
                v-if="(explained.unreadable ?? 0) > 0"
                class="notice-error"
                data-test="explain-unreadable"
              >
                {{ explained.unreadable }} planning document(s) could not be read; this answer may
                be missing records
              </p>
              <template v-if="(explained.blocked_by ?? []).length > 0">
                <h3>Blocked by</h3>
                <ul class="relations">
                  <li
                    v-for="entry in explained.blocked_by"
                    :key="entry.blocker"
                    data-test="explain-blocker"
                  >
                    <RouterLink :to="artifactRoute(entry.blocker)">{{ entry.blocker }}</RouterLink>
                    <span class="relation-name">{{ entry.type }}</span>
                    <span
                      v-if="entry.withholds !== undefined && entry.withholds !== null"
                      class="explain-note"
                      >withholds {{ evidenceText(entry.withholds) }}</span
                    >
                  </li>
                </ul>
              </template>
              <h3>Reached</h3>
              <ul v-if="(explained.reached ?? []).length > 0" class="explain-list">
                <li v-for="(step, index) in explained.reached" :key="index">
                  <strong>{{ step.from }} → {{ step.to }}</strong>
                  <span v-if="step.at" class="explain-at">{{ step.at }}</span>
                  <span v-if="stepDetails(step) !== ''" class="explain-at"
                    >({{ stepDetails(step) }})</span
                  >
                  <ul v-if="(step.rested_on ?? []).length > 0">
                    <li v-for="(evidence, at) in step.rested_on" :key="at">
                      {{ evidenceText(evidence) }}
                    </li>
                  </ul>
                  <p v-if="step.on_nothing_recorded" class="explain-note">
                    {{ step.on_nothing_recorded }}
                  </p>
                </li>
              </ul>
              <p v-else class="plan-loading">no transitions yet</p>
              <template v-if="(explained.recorded_since ?? []).length > 0">
                <h3>Recorded since</h3>
                <ul class="explain-list" data-test="explain-recorded">
                  <li v-for="(record, index) in explained.recorded_since" :key="index">
                    {{ evidenceText(record) }}
                  </li>
                </ul>
              </template>
              <h3>Next</h3>
              <ul class="explain-list" data-test="explain-next">
                <li v-for="next in explained.next ?? []" :key="next.status">
                  <strong>{{ next.status }}</strong>
                  <span v-if="(next.needs ?? []).length === 0" class="explain-note">
                    needs nothing recorded</span
                  >
                  <ul v-else>
                    <li v-for="need in next.needs" :key="need.kind">
                      {{ need.kind }}: {{ need.held ?? 0 }} of {{ need.at_least ?? 1 }}
                    </li>
                  </ul>
                </li>
              </ul>
            </template>
          </section>
        </aside>
      </div>

      <section
        v-if="(artifact.findings ?? []).length > 0"
        class="artifact-section"
        data-test="findings"
      >
        <h2>Findings</h2>
        <table>
          <thead>
            <tr>
              <th>severity</th>
              <th>verdict</th>
              <th>category</th>
              <th>origin</th>
              <th>where</th>
              <th>message</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(finding, index) in artifact.findings" :key="index">
              <td>{{ finding.severity }}</td>
              <td>{{ finding.verdict }}</td>
              <td>{{ finding.category }}</td>
              <td>{{ finding.origin }}</td>
              <td>
                <code>{{ location(finding.file, finding.line) }}</code>
              </td>
              <td>{{ finding.message }}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section
        v-if="(artifact.outcomes ?? []).length > 0"
        class="artifact-section"
        data-test="outcomes"
      >
        <h2>Outcomes</h2>
        <table>
          <thead>
            <tr>
              <th>reviewed</th>
              <th>outcome</th>
              <th>at</th>
              <th>source</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(outcome, index) in artifact.outcomes" :key="index">
              <td>
                <RouterLink v-if="outcome.reviewed" :to="artifactRoute(outcome.reviewed)">{{
                  outcome.reviewed
                }}</RouterLink>
              </td>
              <td>{{ outcome.outcome }}</td>
              <td>{{ outcome.at }}</td>
              <td>{{ outcome.source }}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section class="artifact-section" data-test="history">
        <h2>History</h2>
        <p v-if="history === null" class="plan-loading">Loading…</p>
        <ToolFailureView v-else-if="history.state !== 'ready'" :result="history" />
        <p v-else-if="entries.length === 0" class="plan-loading">no recorded changes</p>
        <ol v-else class="history">
          <li
            v-for="(entry, index) in entries"
            :key="index"
            data-test="history-entry"
            :data-revision="entry.revision"
          >
            <span class="history-revision">r{{ entry.revision }}</span>
            <span class="history-at">{{ entry.at }}</span>
            <strong>{{ historyText(entry) }}</strong>
            <span v-if="entry.actor" class="history-actor">{{ entry.actor }}</span>
          </li>
        </ol>
      </section>
    </article>
  </section>
</template>

<style scoped>
.artifact-title {
  margin: 0.4rem 0;
  font-size: 1.6rem;
}

.artifact-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
}

.artifact-revision,
.artifact-owner {
  color: var(--muted);
  font-size: 0.85rem;
}

.artifact-summary {
  margin: 0 0 var(--gap);
  color: var(--muted);
  font-size: 1.05rem;
}

.artifact-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 20rem;
  gap: calc(var(--gap) * 1.5);
  align-items: start;
}

@media (max-width: 70rem) {
  .artifact-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}

.artifact-body {
  min-width: 0;
  line-height: 1.55;
}

.artifact-body :deep(table) {
  border-collapse: collapse;
}

.artifact-body :deep(th),
.artifact-body :deep(td) {
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border);
  vertical-align: top;
}

.artifact-body :deep(pre) {
  padding: 0.6rem;
  overflow-x: auto;
  border-radius: var(--radius);
  background: var(--surface);
}

.artifact-body :deep(code) {
  font-family: var(--mono);
  font-size: 0.9em;
}

.artifact-side {
  display: flex;
  flex-direction: column;
  gap: var(--gap);
}

.side-section {
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  font-size: 0.9rem;
}

.side-section h2 {
  margin: 0 0 0.4rem;
  font-size: 1rem;
}

.side-section h3 {
  margin: 0.6rem 0 0.25rem;
  font-size: 0.85rem;
  color: var(--muted);
}

.relations,
.scope,
.explain-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.relations li,
.scope li,
.explain-list > li {
  padding: 0.15rem 0;
  overflow-wrap: anywhere;
}

.relation-name,
.scope-confidence,
.explain-at {
  margin: 0 0.4rem;
  color: var(--muted);
  font-size: 0.8rem;
}

.explain-list ul {
  margin: 0.15rem 0 0;
  padding-left: 1rem;
}

.explain-note {
  margin: 0.15rem 0 0;
  color: var(--muted);
  font-size: 0.8rem;
}

.artifact-section {
  margin-top: calc(var(--gap) * 1.5);
}

.artifact-section table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
}

.artifact-section th,
.artifact-section td {
  padding: 0.3rem 0.5rem;
  border-bottom: 1px solid var(--border);
  text-align: left;
  vertical-align: top;
}

.history {
  margin: 0;
  padding-left: 1.2rem;
}

.history li {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  padding: 0.15rem 0;
}

.history-revision,
.history-at,
.history-actor {
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.8rem;
}
</style>
