<script setup lang="ts">
// What the beyond10x codegate on the server's PATH can say: the binary, its version, the
// commands it offers and why there is no assessment. The assessment is never rendered.
import { CODEGATE_REPOSITORY, type QualityReport } from '../../api/quality'

defineProps<{ report: QualityReport }>()
</script>

<template>
  <article class="codegate" data-test="codegate-report">
    <p class="producer" data-test="producer">
      answered by <strong>{{ report.tool }}</strong>
      <span class="version" data-test="tool-version">{{ report.tool_version }}</span>
      <code data-test="tool-path">{{ report.tool_path }}</code>
    </p>
    <p v-if="report.reason !== null" class="notice" data-test="reason">{{ report.reason }}</p>
    <section class="commands">
      <h2>Commands</h2>
      <ul v-if="report.commands.length > 0">
        <li v-for="command in report.commands" :key="command">
          <code data-test="command">{{ command }}</code>
        </li>
      </ul>
      <p v-else class="muted" data-test="no-commands">codegate --help lists no commands</p>
    </section>
    <section v-if="report.skipped.length > 0" class="skipped" data-test="skipped">
      <h2>Passed over</h2>
      <p class="muted">on PATH before it, not the beyond10x codegate:</p>
      <ul>
        <li v-for="path in report.skipped" :key="path">
          <code data-test="skipped-path">{{ path }}</code>
        </li>
      </ul>
    </section>
    <p class="source">
      <a :href="CODEGATE_REPOSITORY" data-test="codegate-link" rel="noopener noreferrer">
        {{ CODEGATE_REPOSITORY }}
      </a>
    </p>
  </article>
</template>

<style scoped>
.codegate {
  max-width: 60rem;
}

.producer {
  margin: 0 0 var(--gap);
  color: var(--muted);
}

.producer .version,
.producer code {
  margin-left: 0.4rem;
}

code {
  font-family: var(--mono);
}

h2 {
  font-size: 1rem;
  margin: var(--gap) 0 0.4rem;
}

ul {
  margin: 0;
  padding-left: 1.2rem;
}

.muted {
  color: var(--muted);
}
</style>
