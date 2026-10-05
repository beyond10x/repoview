<script setup lang="ts">
// The only way repository markdown reaches the DOM: markdown-it with raw HTML off, then DOMPurify.
// A repository is untrusted input and the page holds the run token.
import DOMPurify from 'dompurify'
import MarkdownIt from 'markdown-it'
import { computed } from 'vue'

const props = defineProps<{ source: string }>()

const markdown = new MarkdownIt({ html: false, linkify: true })

const html = computed(() => DOMPurify.sanitize(markdown.render(props.source)))
</script>

<template>
  <div class="markdown" data-test="markdown">
    <!-- eslint-disable-next-line vue/no-v-html -- sanitised by DOMPurify above -->
    <div v-html="html" />
  </div>
</template>
