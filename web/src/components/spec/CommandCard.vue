<script setup lang="ts">
// One command: its input and response, and each outcome with the condition it is taken on, the
// entity it creates, moves or updates and which instance, what it sets, the events it emits and
// the error or response it answers with.
import type { CommandDecl } from '../../api/spec'
import DeclLink from './DeclLink.vue'
import DeclName from './DeclName.vue'
import FieldTable from './FieldTable.vue'
import { assignmentText, conditionText, declAnchor, instanceText } from './ir'

defineProps<{ command: CommandDecl; anchors: ReadonlyMap<string, string> }>()
</script>

<template>
  <article
    :id="declAnchor('command', command.name)"
    class="decl"
    data-test="command"
    :data-command="command.name"
  >
    <DeclName :name="command.name" :naming="command.naming" />
    <FieldTable :fields="command.input" caption="input" />
    <div v-if="command.response.length > 0" data-test="response">
      <FieldTable :fields="command.response" caption="response" />
    </div>
    <h4>outcomes</h4>
    <p v-if="command.outcomes.length === 0" class="none" data-test="no-outcomes">none declared</p>
    <ul v-else class="outcomes">
      <li v-for="(outcome, position) in command.outcomes" :key="position" data-test="outcome">
        <code class="outcome-name">{{ outcome.name }}</code>
        <span v-if="outcome.condition" class="meta">{{ conditionText(outcome.condition) }}</span>
        <span v-if="outcome.subject" class="effect">
          {{ outcome.subject.effect }}
          <DeclLink
            :name="outcome.subject.entity"
            :anchor="anchors.get(outcome.subject.entity) ?? null"
          />
          <template v-if="outcome.subject.transition">
            by <code>{{ outcome.subject.transition.name }}</code> to
            <code>{{ outcome.subject.transition.to }}</code>
          </template>
          <template v-if="outcome.subject.instance">
            ({{ instanceText(outcome.subject.instance) }})</template
          >
        </span>
        <span v-if="outcome.emits.length > 0" class="emits">
          emits
          <template v-for="(event, index) in outcome.emits" :key="event">
            <span v-if="index > 0">, </span>
            <DeclLink :name="event" :anchor="anchors.get(event) ?? null" />
          </template>
        </span>
        <span v-if="outcome.sets.length > 0" class="sets" data-test="sets">
          sets
          <template v-for="(assigned, index) in outcome.sets" :key="index">
            <span v-if="index > 0">, </span>
            <code>{{ assignmentText(assigned) }}</code>
          </template>
        </span>
        <span v-if="outcome.returns" class="meta">returns the response</span>
        <span v-if="outcome.error" class="error">
          error <code :title="outcome.error">{{ outcome.error }}</code></span
        >
        <p v-if="outcome.summary" class="outcome-summary">{{ outcome.summary }}</p>
      </li>
    </ul>
  </article>
</template>

<style scoped>
.decl {
  padding: var(--gap);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  scroll-margin-top: 1rem;
}

.decl:target {
  border-color: var(--accent);
}

.decl + .decl {
  margin-top: var(--gap);
}

h4 {
  margin: 0.75rem 0 0.25rem;
  color: var(--muted);
  font-size: 0.8rem;
  font-weight: 400;
}

.none {
  margin: 0;
  color: var(--muted);
}

.outcomes {
  margin: 0;
  padding-left: 1rem;
  font-size: 0.9rem;
}

.outcomes li + li {
  margin-top: 0.35rem;
}

.meta,
.effect,
.emits,
.sets,
.error {
  margin-left: 0.5rem;
}

.meta {
  color: var(--muted);
}

.error code {
  color: var(--failed);
}

.outcome-summary {
  margin: 0.15rem 0 0;
  color: var(--muted);
}
</style>
