import type { InjectionKey, Ref } from 'vue'

/** "Expand all" / "Collapse all": every node follows `open` each time `serial` changes. */
export interface TreeCommand {
  serial: number
  open: boolean
}

export const TREE_COMMAND: InjectionKey<Readonly<Ref<TreeCommand>>> = Symbol('repoview.plan-tree')
