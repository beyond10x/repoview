<script setup lang="ts">
import type { Remote } from '../../api/repository'

defineProps<{ remotes: Remote[]; error?: string | null }>()
</script>

<template>
  <p v-if="error" class="block-error" data-test="block-error">{{ error }}</p>
  <p v-else-if="remotes.length === 0" class="empty" data-test="empty">no remotes</p>
  <table v-else class="rows">
    <tbody>
      <tr v-for="remote in remotes" :key="remote.name" data-test="remote">
        <td class="nowrap">
          <code>{{ remote.name }}</code>
        </td>
        <td class="mono">{{ remote.url }}</td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped src="./tables.css"></style>
