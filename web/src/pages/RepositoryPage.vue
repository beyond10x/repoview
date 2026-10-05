<script setup lang="ts">
// Git state and the top-level documents: /api/vcs and /api/docs.
import { shallowRef } from 'vue'
import { loadDocuments, loadVcs, type DocumentEntry, type Load, type Vcs } from '../api/repository'
import CommitsTable from '../components/repository/CommitsTable.vue'
import DocumentTabs from '../components/repository/DocumentTabs.vue'
import RemotesList from '../components/repository/RemotesList.vue'
import StatusBlock from '../components/repository/StatusBlock.vue'
import TagsTable from '../components/repository/TagsTable.vue'
import WorktreesTable from '../components/repository/WorktreesTable.vue'

const vcs = shallowRef<Load<Vcs>>({ state: 'loading' })
const documents = shallowRef<Load<DocumentEntry[]>>({ state: 'loading' })

void loadVcs().then((result) => (vcs.value = result))
void loadDocuments().then((result) => (documents.value = result))
</script>

<template>
  <section class="repository" data-test="page" data-page="repository">
    <h1>Repository</h1>
    <div class="blocks">
      <article class="block full" data-block="status">
        <h2>Status</h2>
        <p v-if="vcs.state === 'loading'" class="muted">loading…</p>
        <p
          v-else-if="vcs.state === 'unavailable'"
          class="unavailable"
          :class="`reason-${vcs.reason}`"
          data-test="unavailable"
          data-block="vcs"
          :data-reason="vcs.reason"
        >
          {{ vcs.message }}
        </p>
        <StatusBlock v-else :vcs="vcs.data" />
      </article>

      <template v-if="vcs.state === 'ready'">
        <article class="block" data-block="commits">
          <h2>
            Commits <span class="count">{{ vcs.data.commits.length }} most recent</span>
          </h2>
          <CommitsTable :commits="vcs.data.commits" />
        </article>

        <article class="block" data-block="tags">
          <h2>
            Tags <span class="count">{{ vcs.data.tags.length }} newest</span>
          </h2>
          <TagsTable :tags="vcs.data.tags" :error="vcs.data.tags_error" />
        </article>

        <article class="block" data-block="worktrees">
          <h2>Worktrees</h2>
          <WorktreesTable :worktrees="vcs.data.worktrees" :error="vcs.data.worktrees_error" />
        </article>

        <article class="block" data-block="remotes">
          <h2>Remotes</h2>
          <RemotesList :remotes="vcs.data.remotes" :error="vcs.data.remotes_error" />
        </article>
      </template>

      <article class="block wide" data-block="docs">
        <h2>Documents</h2>
        <p v-if="documents.state === 'loading'" class="muted">loading…</p>
        <p
          v-else-if="documents.state === 'unavailable'"
          class="unavailable"
          :class="`reason-${documents.reason}`"
          data-test="unavailable"
          data-block="docs"
          :data-reason="documents.reason"
        >
          {{ documents.message }}
        </p>
        <DocumentTabs v-else :documents="documents.data" />
      </article>
    </div>
  </section>
</template>

<style scoped>
.blocks {
  display: grid;
  grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
  gap: var(--gap);
  align-items: start;
}

@media (max-width: 70rem) {
  .blocks {
    grid-template-columns: minmax(0, 1fr);
  }
}

.block {
  padding: var(--gap);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  min-width: 0;
}

.block.full,
.block.wide {
  grid-column: 1 / -1;
}

.block.wide {
  background: var(--bg);
}

.block h2 {
  margin: 0 0 0.75rem;
  font-size: 1.05rem;
}

.count {
  margin-left: 0.4rem;
  color: var(--muted);
  font-size: 0.85rem;
  font-weight: normal;
}

.muted {
  color: var(--muted);
}

.unavailable {
  margin: 0;
}

.reason-absent {
  color: var(--absent);
}

.reason-error {
  color: var(--failed);
  font-weight: 600;
}
</style>
