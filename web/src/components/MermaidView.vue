<script lang="ts">
// Shared by every instance: Mermaid renders through a temporary element with this id.
let nextId = 0
</script>

<script setup lang="ts">
// A Mermaid diagram. Mermaid runs with securityLevel "strict" and is loaded on first use, so it
// stays out of the main bundle. Its SVG is sanitised again with DOMPurify and inserted as nodes.
// A failure shows the error and the source; the box is never blank.
import DOMPurify from 'dompurify'
import { onBeforeUnmount, ref, shallowRef, useTemplateRef, watch } from 'vue'

const props = defineProps<{ source: string }>()

type State = { state: 'rendering' } | { state: 'ready' } | { state: 'error'; message: string }

const state = shallowRef<State>({ state: 'rendering' })
const svg = ref<string | null>(null)
const container = useTemplateRef<HTMLDivElement>('container')

let current = 0
let initialised = false

async function renderDiagram(source: string): Promise<void> {
  const run = ++current
  state.value = { state: 'rendering' }
  svg.value = null
  if (source.trim() === '') {
    state.value = { state: 'error', message: 'empty diagram source' }
    return
  }
  try {
    const { default: mermaid } = await import('mermaid')
    if (!initialised) {
      mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' })
      initialised = true
    }
    const { svg: rendered } = await mermaid.render(`repoview-mermaid-${String(++nextId)}`, source)
    if (run !== current) return
    svg.value = rendered
    state.value = { state: 'ready' }
  } catch (error) {
    if (run !== current) return
    state.value = {
      state: 'error',
      message: error instanceof Error ? error.message : String(error),
    }
  }
}

watch(() => props.source, renderDiagram, { immediate: true })

watch([svg, container], ([markup, element]) => {
  if (element === null) return
  if (markup === null) {
    element.replaceChildren()
    return
  }
  element.replaceChildren(
    DOMPurify.sanitize(markup, {
      USE_PROFILES: { svg: true, svgFilters: true, html: true },
      // Mermaid's HTML labels live in foreignObject.
      ADD_TAGS: ['foreignObject'],
      HTML_INTEGRATION_POINTS: { foreignobject: true },
      RETURN_DOM_FRAGMENT: true,
    }),
  )
})

onBeforeUnmount(() => {
  current = -1
})
</script>

<template>
  <figure class="mermaid" data-test="mermaid">
    <div v-show="state.state === 'ready'" ref="container" class="mermaid-svg" />
    <p v-if="state.state === 'rendering'" class="notice">rendering diagram…</p>
    <template v-else-if="state.state === 'error'">
      <p class="notice notice-error" role="alert">diagram failed to render</p>
      <pre class="diagnostic" data-test="mermaid-error"
        >{{ state.message }}

{{ source }}</pre>
    </template>
  </figure>
</template>
