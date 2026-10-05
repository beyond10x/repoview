<script setup lang="ts">
import { humanise, scoreBand, type ScoreRow } from '../../api/quality'

// `max`: the assessment's `score_max`, or null when it gave none (then bars stay empty).
defineProps<{ rows: ScoreRow[]; max: number | null }>()

function width(fraction: number | null): string {
  return `${String(Math.round((fraction ?? 0) * 1000) / 10)}%`
}
</script>

<template>
  <ul class="scores">
    <li
      v-for="row in rows"
      :key="row.name"
      class="score"
      :class="{ 'score-overall': row.name === 'overall' }"
      data-test="score"
      :data-score="row.name"
    >
      <span class="label">{{ humanise(row.name) }}</span>
      <span
        class="track"
        role="meter"
        :aria-label="humanise(row.name)"
        aria-valuemin="0"
        :aria-valuemax="max ?? undefined"
        :aria-valuenow="row.value"
      >
        <span
          class="fill"
          :class="`band-${scoreBand(row.fraction)}`"
          :style="{ width: width(row.fraction) }"
        ></span>
      </span>
      <span class="value">{{ row.value }}</span>
    </li>
  </ul>
</template>

<style scoped>
.scores {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.35rem;
}

.score {
  display: grid;
  grid-template-columns: 9rem 1fr 2.5rem;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
}

.score-overall {
  font-weight: 600;
  margin-bottom: 0.25rem;
}

.label {
  color: var(--muted);
}

.score-overall .label {
  color: var(--fg);
}

.track {
  display: block;
  height: 0.55rem;
  border-radius: 999px;
  background: var(--border);
  overflow: hidden;
}

.score-overall .track {
  height: 0.8rem;
}

.fill {
  display: block;
  height: 100%;
  border-radius: 999px;
  background: var(--muted);
}

.band-good {
  background: var(--present);
}

.band-fair {
  background: var(--missing);
}

.band-poor {
  background: var(--failed);
}

.value {
  font-family: var(--mono);
  text-align: right;
}
</style>
