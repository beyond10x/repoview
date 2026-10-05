<script setup lang="ts">
import { shortDate, shortSha, type Tag } from '../../api/repository'

defineProps<{ tags: Tag[]; error?: string | null }>()
</script>

<template>
  <p v-if="error" class="block-error" data-test="block-error">{{ error }}</p>
  <p v-else-if="tags.length === 0" class="empty" data-test="empty">no tags</p>
  <table v-else class="rows">
    <thead>
      <tr>
        <th scope="col">tag</th>
        <th scope="col">commit</th>
        <th scope="col">date</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="tag in tags" :key="tag.name" data-test="tag">
        <td>
          <code data-test="tag-name">{{ tag.name }}</code>
        </td>
        <td class="mono">
          <code data-test="tag-sha" :title="tag.sha">{{ shortSha(tag.sha) }}</code>
        </td>
        <td class="nowrap muted">
          <time :datetime="tag.date" data-test="tag-date">{{ shortDate(tag.date) }}</time>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped src="./tables.css"></style>
