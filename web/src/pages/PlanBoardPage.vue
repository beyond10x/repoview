<script setup lang="ts">
// The plan as status columns, exactly as `aep plan artifact board` orders and names them, with
// `aep plan artifact validate` in the header.
import { computed, ref, shallowRef } from 'vue'
import type { ApiResult } from '../api/client'
import {
  filterColumns,
  kindsOf,
  loadBoard,
  loadValidate,
  validation,
  type BoardColumn,
  type Validate,
} from '../api/plan'
import PlanCard from '../components/plan/PlanCard.vue'
import ToolFailureView from '../components/plan/ToolFailureView.vue'
import '../components/plan/plan.css'

const board = shallowRef<ApiResult<BoardColumn[]> | null>(null)
const validated = shallowRef<ApiResult<Validate> | null>(null)

void loadBoard().then((loaded) => {
  board.value = loaded
})
void loadValidate().then((loaded) => {
  validated.value = loaded
})

const text = ref('')
const kind = ref('')

const columns = computed(() => (board.value?.state === 'ready' ? board.value.data : []))
const kinds = computed(() => kindsOf(columns.value))
const shown = computed(() => filterColumns(columns.value, text.value, kind.value))
const total = computed(() => columns.value.reduce((sum, c) => sum + c.artifacts.length, 0))
const matching = computed(() => shown.value.reduce((sum, c) => sum + c.artifacts.length, 0))

const check = computed(() => (validated.value === null ? null : validation(validated.value)))
</script>

<template>
  <section data-test="page" data-page="plan-board">
    <header class="plan-header">
      <h1>Plan · Board</h1>
      <nav class="plan-views">
        <RouterLink to="/plan">Board</RouterLink>
        <RouterLink to="/plan/tree">Tree</RouterLink>
      </nav>
    </header>

    <div class="board-validate">
      <span class="board-validate-label">aep validate</span>
      <span v-if="check === null" class="plan-loading">checking…</span>
      <span v-else-if="check.state === 'valid'" class="validate-valid" data-test="validate"
        >valid</span
      >
      <div v-else-if="check.state === 'problems'" class="validate-problems" data-test="validate">
        <strong>{{ check.lines.length }} problem{{ check.lines.length === 1 ? '' : 's' }}</strong>
        <ul>
          <li v-for="(line, index) in check.lines" :key="index" data-test="validate-problem">
            {{ line }}
          </li>
        </ul>
      </div>
      <pre v-else class="diagnostic" data-test="validate">{{ check.text }}</pre>
    </div>

    <p v-if="board === null" class="plan-loading">Loading the board…</p>
    <ToolFailureView v-else-if="board.state !== 'ready'" :result="board" />
    <template v-else>
      <div class="board-filters">
        <input
          v-model="text"
          type="search"
          placeholder="Filter by title, id or tag"
          aria-label="Filter by title, id or tag"
          data-test="board-filter"
        />
        <select v-model="kind" aria-label="Kind" data-test="kind-filter">
          <option value="">all kinds</option>
          <option v-for="name in kinds" :key="name" :value="name">{{ name }}</option>
        </select>
        <span class="board-matching">{{ matching }} of {{ total }}</span>
      </div>
      <div class="board">
        <section
          v-for="column in shown"
          :key="column.status"
          class="board-column"
          data-test="board-column"
          :data-status="column.status"
        >
          <header class="board-column-header">
            <h2 data-test="column-status">{{ column.status }}</h2>
            <span class="board-column-count" data-test="column-count">{{
              column.artifacts.length
            }}</span>
          </header>
          <p
            v-if="column.description !== undefined && column.description !== null"
            class="board-column-description"
            data-test="column-description"
          >
            {{ column.description }}
          </p>
          <ul class="board-cards">
            <PlanCard
              v-for="artifact in column.artifacts"
              :key="artifact.id"
              :artifact="artifact"
            />
          </ul>
        </section>
      </div>
    </template>
  </section>
</template>

<style scoped>
.board-validate {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
  margin-bottom: var(--gap);
}

.board-validate-label {
  font-family: var(--mono);
  font-size: 0.8rem;
  color: var(--muted);
}

.validate-valid {
  color: var(--present);
  font-weight: 600;
}

.validate-problems {
  color: var(--failed);
}

.validate-problems ul {
  margin: 0.25rem 0 0;
  padding-left: 1.2rem;
  font-family: var(--mono);
  font-size: 0.8rem;
  white-space: pre-wrap;
}

.board-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: var(--gap);
}

.board-filters input,
.board-filters select {
  padding: 0.3rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--fg);
  font: inherit;
}

.board-filters input {
  min-width: 18rem;
}

.board-matching {
  color: var(--muted);
  font-size: 0.85rem;
}

.board {
  display: flex;
  align-items: flex-start;
  gap: var(--gap);
  overflow-x: auto;
  padding-bottom: 0.5rem;
}

.board-column {
  flex: 0 0 17rem;
  padding: 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}

.board-column-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.board-column-header h2 {
  margin: 0;
  font-size: 1rem;
}

.board-column-count {
  color: var(--muted);
  font-size: 0.85rem;
}

.board-column-description {
  margin: 0.3rem 0 0;
  color: var(--muted);
  font-size: 0.8rem;
}

.board-cards {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0.6rem 0 0;
  padding: 0;
}
</style>
