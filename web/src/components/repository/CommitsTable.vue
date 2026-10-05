<script setup lang="ts">
import { shortDate, shortSha, type Commit } from '../../api/repository'

defineProps<{ commits: Commit[] }>()
</script>

<template>
  <p v-if="commits.length === 0" class="empty" data-test="empty">no commits yet</p>
  <table v-else class="rows">
    <thead>
      <tr>
        <th scope="col">commit</th>
        <th scope="col">subject</th>
        <th scope="col">author</th>
        <th scope="col">date</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="commit in commits" :key="commit.sha" data-test="commit">
        <td class="mono nowrap">
          <code data-test="commit-sha" :title="commit.sha">{{ shortSha(commit.sha) }}</code>
        </td>
        <td data-test="commit-subject">{{ commit.subject }}</td>
        <td class="nowrap" data-test="commit-author">{{ commit.author }}</td>
        <td class="nowrap muted">
          <time :datetime="commit.date" data-test="commit-date">{{ shortDate(commit.date) }}</time>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped src="./tables.css"></style>
