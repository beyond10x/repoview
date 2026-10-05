<script setup lang="ts">
// One entity: identity, fields with types, relations as links to their targets, invariants, and
// the lifecycle as a state diagram built from the IR.
import { computed } from 'vue'
import type { CommandDecl, Entity } from '../../api/spec'
import MermaidView from '../MermaidView.vue'
import DeclLink from './DeclLink.vue'
import DeclName from './DeclName.vue'
import FieldTable from './FieldTable.vue'
import { entityAnchor, lifecycleSource, typeLabel } from './ir'

const props = defineProps<{
  entity: Entity
  commands: Record<string, CommandDecl>
  /** Declaration name → element id, for every card on the page. */
  anchors: ReadonlyMap<string, string>
}>()

const lifecycle = computed(() => lifecycleSource(props.entity, props.commands))
const relations = computed(() => props.entity.relations)
const invariants = computed(() =>
  props.entity.invariants.map((item) => (typeof item === 'string' ? item : JSON.stringify(item))),
)
</script>

<template>
  <article
    :id="entityAnchor(entity.name)"
    class="entity"
    data-test="entity"
    :data-entity="entity.name"
  >
    <DeclName :name="entity.name" :naming="entity.naming" />
    <p v-if="entity.identity" class="identity" data-test="identity">
      identified by <code>{{ entity.identity.name }}</code
      >: <code class="type">{{ typeLabel(entity.identity.type_ref) }}</code>
    </p>
    <p v-else class="identity none" data-test="identity">identity: none declared</p>
    <div class="body">
      <div class="facts">
        <FieldTable :fields="entity.fields" caption="fields" />
        <div v-if="relations.length > 0" class="relations">
          <h4>relations</h4>
          <ul>
            <li
              v-for="relation in relations"
              :key="relation.name"
              data-test="relation"
              :data-relation="relation.name"
            >
              <code>{{ relation.name }}</code>
              <span class="relation-kind">{{ relation.kind }}</span>
              <DeclLink
                v-if="relation.target !== undefined"
                :name="relation.target"
                :anchor="anchors.get(relation.target) ?? null"
              />
              <span v-else class="none">no target</span>
              <span v-if="relation.cardinality" class="relation-meta">{{
                relation.cardinality
              }}</span>
              <span v-if="relation.via" class="relation-meta">
                via <code>{{ relation.via }}</code></span
              >
            </li>
          </ul>
        </div>
        <div v-if="invariants.length > 0" class="invariants">
          <h4>invariants</h4>
          <ul>
            <li v-for="(invariant, index) in invariants" :key="index">
              <code>{{ invariant }}</code>
            </li>
          </ul>
        </div>
      </div>
      <div v-if="lifecycle" class="lifecycle" data-test="lifecycle">
        <h4>lifecycle</h4>
        <MermaidView :source="lifecycle" />
      </div>
      <div v-else class="lifecycle" data-test="no-lifecycle">
        <h4>lifecycle</h4>
        <p class="none">none declared</p>
      </div>
    </div>
  </article>
</template>

<style scoped>
.entity {
  padding: var(--gap);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  scroll-margin-top: 1rem;
}

.entity:target {
  border-color: var(--accent);
}

.entity + .entity {
  margin-top: var(--gap);
}

.identity {
  margin: 0.5rem 0;
  font-size: 0.9rem;
}

.type {
  color: var(--accent);
}

.body {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: var(--gap);
}

@media (max-width: 70rem) {
  .body {
    grid-template-columns: minmax(0, 1fr);
  }
}

h4 {
  margin: 0.75rem 0 0.25rem;
  color: var(--muted);
  font-size: 0.8rem;
  font-weight: 400;
}

.lifecycle h4 {
  margin-top: 0;
}

ul {
  margin: 0;
  padding-left: 1rem;
  font-size: 0.9rem;
}

.relation-kind,
.relation-meta {
  margin-left: 0.4rem;
  color: var(--muted);
}

.relation-kind {
  margin-right: 0.4rem;
}

.none {
  color: var(--muted);
}
</style>
