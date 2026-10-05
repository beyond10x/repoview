<script setup lang="ts">
// The seven sections of a spec root page over its compiled IR and interaction graph.
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import type { ApiResult } from '../../api/client'
import type { Graph, Ir } from '../../api/spec'
import MermaidView from '../MermaidView.vue'
import CommandCard from './CommandCard.vue'
import DeclLink from './DeclLink.vue'
import DeclName from './DeclName.vue'
import EntityCard from './EntityCard.vue'
import FieldTable from './FieldTable.vue'
import SpecSection from './SpecSection.vue'
import { declAnchor, entityAnchor } from './ir'

const props = defineProps<{
  ir: Ir
  graph: ApiResult<Graph> | null
  mermaid: ApiResult<string> | null
}>()
const route = useRoute()

function entries<T>(record: Record<string, T> | undefined | null): T[] {
  return record === undefined || record === null ? [] : Object.values(record)
}

const domains = computed(() => entries(props.ir.domains))
const entities = computed(() => entries(props.ir.entities))
const commands = computed(() => entries(props.ir.commands))
const events = computed(() => entries(props.ir.events))
const views = computed(() => entries(props.ir.views))
const components = computed(() => entries(props.ir.components))
const commandMap = computed(() => props.ir.commands)

/** Every declaration with a card on this page, by name. */
const anchors = computed(() => {
  const map = new Map<string, string>()
  for (const entity of entities.value) map.set(entity.name, entityAnchor(entity.name))
  for (const command of commands.value) map.set(command.name, declAnchor('command', command.name))
  for (const event of events.value) map.set(event.name, declAnchor('event', event.name))
  for (const view of views.value) map.set(view.name, declAnchor('view', view.name))
  for (const component of components.value) {
    map.set(component.name, declAnchor('component', component.name))
  }
  return map
})

function count(list: string[], one: string, many: string): string | null {
  const n = list.length
  return n === 0 ? null : `${String(n)} ${n === 1 ? one : many}`
}

function domainCounts(domain: Ir['domains'][string]): string {
  const parts = [
    count(domain.entities, 'entity', 'entities'),
    count(domain.commands, 'command', 'commands'),
    count(domain.events, 'event', 'events'),
    count(domain.views, 'view', 'views'),
    count(domain.types, 'type', 'types'),
    count(domain.errors, 'error', 'errors'),
    count(domain.actors, 'actor', 'actors'),
  ].filter((part): part is string => part !== null)
  return parts.length === 0 ? 'nothing declared' : parts.join(' · ')
}

const graphReady = computed(() => (props.graph?.state === 'ready' ? props.graph.data : null))
const graphEmpty = computed(() => graphReady.value !== null && graphReady.value.nodes.length === 0)
const graphCount = computed(() => graphReady.value?.nodes.length ?? 0)

function failure(result: ApiResult<unknown> | null): string | null {
  if (result === null || result.state === 'ready') return null
  return result.state === 'token-rejected' ? 'the server rejected the run token' : result.message
}
</script>

<template>
  <nav class="toc" data-test="spec-toc">
    <RouterLink :to="{ path: route.path, hash: '#section-domains' }">Domains</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-entities' }">Entities</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-commands' }">Commands</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-events' }">Events</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-views' }">Views</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-components' }">Components</RouterLink>
    <RouterLink :to="{ path: route.path, hash: '#section-graph' }">Interaction graph</RouterLink>
  </nav>

  <SpecSection id="domains" title="Domains" :count="domains.length">
    <div class="grid">
      <article v-for="domain in domains" :key="domain.name" class="decl" data-test="domain">
        <DeclName :name="domain.name" :naming="domain.naming" />
        <p class="counts">{{ domainCounts(domain) }}</p>
      </article>
    </div>
  </SpecSection>

  <SpecSection id="entities" title="Entities" :count="entities.length">
    <EntityCard
      v-for="entity in entities"
      :key="entity.name"
      :entity="entity"
      :commands="commandMap"
      :anchors="anchors"
    />
  </SpecSection>

  <SpecSection id="commands" title="Commands" :count="commands.length">
    <CommandCard
      v-for="command in commands"
      :key="command.name"
      :command="command"
      :anchors="anchors"
    />
  </SpecSection>

  <SpecSection id="events" title="Events" :count="events.length">
    <div class="grid">
      <article
        v-for="event in events"
        :id="declAnchor('event', event.name)"
        :key="event.name"
        class="decl"
        data-test="event"
      >
        <DeclName :name="event.name" :naming="event.naming" />
        <FieldTable :fields="event.fields" />
      </article>
    </div>
  </SpecSection>

  <SpecSection id="views" title="Views" :count="views.length">
    <div class="grid">
      <article
        v-for="view in views"
        :id="declAnchor('view', view.name)"
        :key="view.name"
        class="decl"
        data-test="view"
        :data-view="view.name"
      >
        <DeclName :name="view.name" :naming="view.naming" />
        <p class="counts">
          <template v-if="view.source">
            from <DeclLink :name="view.source" :anchor="anchors.get(view.source) ?? null" />
          </template>
          <template v-if="view.consistency"> · {{ view.consistency }}</template>
        </p>
        <dl v-if="view.filter !== undefined || view.order_by.length > 0" class="members">
          <template v-if="view.filter !== undefined">
            <dt>filter</dt>
            <dd>
              <code data-test="view-filter">{{ view.filter }}</code>
            </dd>
          </template>
          <template v-if="view.order_by.length > 0">
            <dt>order by</dt>
            <dd>
              <code data-test="view-order">{{ view.order_by.join(', ') }}</code>
            </dd>
          </template>
        </dl>
        <FieldTable :fields="view.fields" />
      </article>
    </div>
  </SpecSection>

  <SpecSection id="components" title="Components" :count="components.length">
    <div class="grid">
      <article
        v-for="component in components"
        :id="declAnchor('component', component.name)"
        :key="component.name"
        class="decl"
        data-test="component"
      >
        <DeclName :name="component.name" :naming="component.naming" />
        <dl class="members">
          <template v-if="component.reached_by !== undefined">
            <dt>reached by</dt>
            <dd>{{ component.reached_by }}</dd>
          </template>
          <template v-if="component.owns.length > 0">
            <dt>owns</dt>
            <dd>
              <code v-for="domain in component.owns" :key="domain">{{ domain }}</code>
            </dd>
          </template>
          <template v-if="component.accepts.length > 0">
            <dt>accepts</dt>
            <dd>
              <DeclLink
                v-for="name in component.accepts"
                :key="name"
                :name="name"
                :anchor="anchors.get(name) ?? null"
              />
            </dd>
          </template>
          <template v-if="component.publishes.length > 0">
            <dt>publishes</dt>
            <dd>
              <DeclLink
                v-for="name in component.publishes"
                :key="name"
                :name="name"
                :anchor="anchors.get(name) ?? null"
              />
            </dd>
          </template>
        </dl>
      </article>
    </div>
  </SpecSection>

  <section id="section-graph" class="spec-graph" data-test="spec-section" data-section="graph">
    <h2>Interaction graph</h2>
    <p v-if="graph === null || mermaid === null" class="notice">loading graph…</p>
    <p v-else-if="failure(graph) ?? failure(mermaid)" class="notice notice-error" role="alert">
      interaction graph unavailable: {{ failure(graph) ?? failure(mermaid) }}
    </p>
    <p v-else-if="graphEmpty" class="none">
      no interactions declared: ess reports no commands, events or actors to connect
    </p>
    <template v-else>
      <p class="counts">
        {{ graphCount }} nodes · {{ graphReady?.edges.length ?? 0 }} edges, as
        <code>ess specify graph</code> draws them
      </p>
      <MermaidView v-if="mermaid.state === 'ready'" :source="mermaid.data" />
      <table v-if="(graphReady?.edges.length ?? 0) > 0" class="edges">
        <thead>
          <tr>
            <th>from</th>
            <th>edge</th>
            <th>to</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(edge, index) in graphReady?.edges ?? []" :key="index" data-test="edge">
            <td><DeclLink :name="edge.from" :anchor="anchors.get(edge.from) ?? null" /></td>
            <td class="edge-kind">{{ edge.kind }}{{ edge.label ? `: ${edge.label}` : '' }}</td>
            <td><DeclLink :name="edge.to" :anchor="anchors.get(edge.to) ?? null" /></td>
          </tr>
        </tbody>
      </table>
    </template>
  </section>
</template>

<style scoped>
.toc {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem 1rem;
  margin: 0.5rem 0 0;
  font-size: 0.9rem;
}

.toc a {
  color: var(--accent);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr));
  gap: var(--gap);
}

.decl {
  padding: var(--gap);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  scroll-margin-top: 1rem;
}

.decl:target {
  border-color: var(--accent);
}

.counts {
  margin: 0.4rem 0;
  color: var(--muted);
  font-size: 0.9rem;
}

.members {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.25rem 0.75rem;
  margin: 0.5rem 0 0;
  font-size: 0.9rem;
}

.members dt {
  color: var(--muted);
}

.members dd {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem 0.75rem;
  margin: 0;
}

.spec-graph {
  margin-top: 2rem;
}

.spec-graph h2 {
  margin: 0 0 0.75rem;
  padding-bottom: 0.25rem;
  border-bottom: 1px solid var(--border);
  font-size: 1.2rem;
}

.none {
  color: var(--muted);
}

.edges {
  margin-top: var(--gap);
  border-collapse: collapse;
  font-size: 0.9rem;
}

.edges th {
  color: var(--muted);
  font-weight: 400;
  text-align: left;
}

.edges th,
.edges td {
  padding: 0.2rem 1rem 0.2rem 0;
  border-top: 1px solid var(--border);
}

.edge-kind {
  color: var(--muted);
}
</style>
