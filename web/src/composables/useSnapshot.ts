import { inject, provide, shallowRef, type InjectionKey, type Ref } from 'vue'
import { loadSnapshot, type SnapshotState } from '../api/snapshot'

const SNAPSHOT_KEY: InjectionKey<Readonly<Ref<SnapshotState>>> = Symbol('repoview.snapshot')

/**
 * The project snapshot. The first caller in a component tree (the app shell) fetches it once and
 * provides it; every descendant that calls `useSnapshot()` shares that state.
 */
export function useSnapshot(): Readonly<Ref<SnapshotState>> {
  const shared = inject(SNAPSHOT_KEY, null)
  if (shared !== null) return shared
  const state = shallowRef<SnapshotState>({ state: 'loading' })
  loadSnapshot().then(
    (loaded) => {
      state.value = loaded
    },
    (error: unknown) => {
      state.value = { state: 'error', message: `snapshot unavailable: ${String(error)}` }
    },
  )
  provide(SNAPSHOT_KEY, state)
  return state
}
