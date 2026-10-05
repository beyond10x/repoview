<script setup lang="ts">
import { computed } from 'vue'
import {
  countRows,
  findingLocation,
  findingSeverity,
  findingTitle,
  humanise,
  isRecord,
  rating as ratingOf,
  ratingBand,
  scoreMax as scoreMaxOf,
  scoreRows,
  summaryRows,
  topFindings,
  type LanguageQuality,
} from '../../api/quality'
import { orElse } from '../../display'
import ScoreBars from './ScoreBars.vue'

const props = defineProps<{ entry: LanguageQuality }>()

// Only an assessed language shows anything from its assessment: a score for a language the tool
// did not assess is never shown, whatever the server sent alongside it.
const assessment = computed(() =>
  props.entry.status === 'assessed' && isRecord(props.entry.assessment)
    ? props.entry.assessment
    : null,
)

const statusLabel = computed(() => {
  switch (props.entry.status) {
    case 'assessed':
      return 'assessed'
    case 'not-assessed':
      return 'not assessed'
    case 'failed':
      return 'failed'
    case 'running':
      return 'assessing…'
  }
  return `unknown status: ${String(props.entry.status)}`
})

const scoreMax = computed(() => (assessment.value === null ? null : scoreMaxOf(assessment.value)))
const scores = computed(() => (assessment.value === null ? [] : scoreRows(assessment.value)))
const counts = computed(() => (assessment.value === null ? [] : countRows(assessment.value)))
const summary = computed(() => (assessment.value === null ? [] : summaryRows(assessment.value)))
const findings = computed(() => (assessment.value === null ? [] : topFindings(assessment.value)))
const rating = computed(() => (assessment.value === null ? null : ratingOf(assessment.value)))
</script>

<template>
  <article
    class="language-card"
    data-test="language-card"
    :data-language="entry.language"
    :data-status="entry.status"
  >
    <header class="head">
      <h2>{{ orElse(entry.language, 'unnamed language') }}</h2>
      <span class="status" :class="`status-${entry.status}`">
        <span
          v-if="entry.status === 'running'"
          class="spinner"
          data-test="spinner"
          aria-hidden="true"
        ></span>
        {{ statusLabel }}
      </span>
      <span
        v-if="assessment !== null"
        class="rating"
        :class="`rating-${ratingBand(rating)}`"
        data-test="rating"
        :title="'rating'"
        >{{ orElse(rating, '–') }}</span
      >
    </header>

    <p v-if="entry.status === 'running'" class="note" role="status">
      codegate is assessing {{ entry.language }}; this page refreshes every 2 s.
    </p>

    <template v-else-if="entry.status === 'not-assessed' || entry.status === 'failed'">
      <p class="note" data-test="reason">{{ orElse(entry.reason, 'no reason given') }}</p>
      <pre v-if="entry.status === 'failed'" class="stderr" data-test="stderr">{{
        orElse(entry.stderr, 'the tool gave no stderr')
      }}</pre>
    </template>

    <template v-else-if="assessment !== null">
      <section v-if="scores.length > 0" class="block">
        <h3>
          Scores <span v-if="scoreMax !== null" class="hint">0–{{ scoreMax }}</span>
        </h3>
        <ScoreBars :rows="scores" :max="scoreMax" />
      </section>

      <div class="columns">
        <section v-if="counts.length > 0" class="block">
          <h3>Finding counts</h3>
          <table class="counts">
            <tbody>
              <tr
                v-for="row in counts"
                :key="row.kind"
                data-test="finding-count"
                :data-kind="row.kind"
              >
                <td>{{ humanise(row.kind) }}</td>
                <td class="num">{{ row.count }}</td>
              </tr>
            </tbody>
          </table>
        </section>

        <section v-if="summary.length > 0" class="block">
          <h3>Summary</h3>
          <dl class="summary" data-test="summary">
            <template v-for="[key, value] in summary" :key="key">
              <dt>{{ humanise(key) }}</dt>
              <dd>{{ value }}</dd>
            </template>
          </dl>
        </section>
      </div>

      <section v-if="findings.length > 0" class="block">
        <h3>Top findings</h3>
        <ol class="findings">
          <li v-for="(finding, index) in findings" :key="index" data-test="top-finding">
            <span
              v-if="findingSeverity(finding) !== null"
              class="severity"
              :class="`severity-${findingSeverity(finding)}`"
              >{{ findingSeverity(finding) }}</span
            >
            <span class="finding-title" data-test="finding-title">{{ findingTitle(finding) }}</span>
            <code
              v-if="findingLocation(finding) !== null"
              class="location"
              data-test="finding-location"
              >{{ findingLocation(finding) }}</code
            >
          </li>
        </ol>
      </section>
    </template>
  </article>
</template>

<style scoped>
.language-card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  padding: var(--gap);
  display: grid;
  gap: 0.9rem;
}

.head {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.head h2 {
  margin: 0;
  font-size: 1.2rem;
  text-transform: capitalize;
}

.status {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
  color: var(--muted);
}

.status-failed {
  color: var(--failed);
}

.status-not-assessed {
  color: var(--absent);
}

.rating {
  margin-left: auto;
  min-width: 3.2rem;
  padding: 0.25rem 0.6rem;
  border-radius: var(--radius);
  font-size: 1.6rem;
  font-weight: 700;
  text-align: center;
  color: var(--bg);
  background: var(--muted);
}

.rating-a {
  background: var(--present);
}

.rating-b {
  background: var(--accent);
}

.rating-c {
  background: var(--missing);
}

.rating-d {
  background: var(--failed);
}

.spinner {
  width: 0.85rem;
  height: 0.85rem;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.note {
  margin: 0;
  color: var(--muted);
}

.stderr {
  margin: 0;
  padding: 0.6rem;
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--failed);
  font-family: var(--mono);
  font-size: 0.85rem;
  white-space: pre-wrap;
  overflow-x: auto;
}

.block h3 {
  margin: 0 0 0.5rem;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--muted);
}

.hint {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 400;
}

.columns {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: var(--gap);
}

.counts {
  border-collapse: collapse;
  width: 100%;
  font-size: 0.9rem;
}

.counts td {
  padding: 0.15rem 0;
  border-bottom: 1px solid var(--border);
}

.num {
  text-align: right;
  font-family: var(--mono);
}

.summary {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.15rem 0.75rem;
  margin: 0;
  font-size: 0.9rem;
}

.summary dt {
  color: var(--muted);
}

.summary dd {
  margin: 0;
  font-family: var(--mono);
}

.findings {
  margin: 0;
  padding-left: 1.4rem;
  display: grid;
  gap: 0.4rem;
  font-size: 0.9rem;
}

.findings li {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.2rem 0.5rem;
}

.severity {
  font-size: 0.75rem;
  padding: 0 0.35rem;
  border-radius: var(--radius);
  border: 1px solid currentColor;
  color: var(--muted);
}

.severity-warning {
  color: var(--missing);
}

.severity-error {
  color: var(--failed);
}

.location {
  color: var(--muted);
  font-family: var(--mono);
  font-size: 0.8rem;
  word-break: break-all;
}
</style>
