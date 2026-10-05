<script setup lang="ts">
// Fields with their types, as a table; a declared type links to nothing (types have no card).
import type { Field } from '../../api/spec'
import { typeLabel } from './ir'

defineProps<{ fields: Field[]; caption?: string }>()
</script>

<template>
  <table v-if="fields.length > 0" class="fields">
    <caption v-if="caption">
      {{
        caption
      }}
    </caption>
    <tbody>
      <tr v-for="field in fields" :key="field.name" data-test="field" :data-field="field.name">
        <td class="field-name">
          <code>{{ field.name }}</code>
        </td>
        <td class="field-type" data-test="type">
          <code :title="JSON.stringify(field.type_ref)">{{ typeLabel(field.type_ref) }}</code>
        </td>
        <td class="field-summary">{{ field.naming?.summary ?? '' }}</td>
      </tr>
    </tbody>
  </table>
  <p v-else class="no-fields">no fields</p>
</template>

<style scoped>
.fields {
  border-collapse: collapse;
  width: 100%;
  font-size: 0.9rem;
}

caption {
  text-align: left;
  color: var(--muted);
  font-size: 0.8rem;
  padding-bottom: 0.25rem;
}

td {
  padding: 0.2rem 0.75rem 0.2rem 0;
  border-top: 1px solid var(--border);
  vertical-align: top;
}

.field-name {
  width: 1%;
  white-space: nowrap;
}

.field-type code {
  color: var(--accent);
}

.field-summary {
  color: var(--muted);
}

.no-fields {
  margin: 0;
  color: var(--muted);
  font-size: 0.9rem;
}
</style>
