import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { Tasks } from '../../api/repository'
import TasksList from './TasksList.vue'

// story:taskfile-tasks acceptance 2: the task list.

const tasks: Tasks = {
  file: 'Taskfile.yml',
  tasks: [
    {
      name: 'check',
      desc: 'The <b>gate</b>',
      summary: 'Runs <i>every</i> check.\nTwice.\n',
      internal: false,
      aliases: ['c', 'gate'],
    },
    { name: 'docs:build', desc: null, summary: null, internal: true, aliases: [] },
  ],
  refused: [
    {
      include: 'remote',
      taskfile: 'https://example.invalid/Taskfile.yml',
      reason: 'remote Taskfile; only files inside the project are read',
    },
  ],
  truncated: false,
}

describe('TasksList', () => {
  it('lists each task with its name, aliases and description', () => {
    const wrapper = mount(TasksList, { props: { tasks } })
    const rows = wrapper.findAll('[data-test="task"]')
    expect(rows).toHaveLength(2)
    const first = rows[0]
    if (first === undefined) throw new Error('no first row')
    expect(first.get('[data-test="task-name"]').text()).toBe('check')
    expect(first.get('[data-test="task-aliases"]').text()).toBe('c, gate')
    expect(first.get('[data-test="task-desc"]').text()).toBe('The <b>gate</b>')
    expect(first.find('[data-test="task-internal"]').exists()).toBe(false)
  })

  it('renders descriptions and summaries as text, never as markup', () => {
    const wrapper = mount(TasksList, { props: { tasks } })
    expect(wrapper.find('b').exists()).toBe(false)
    expect(wrapper.find('i').exists()).toBe(false)
    expect(wrapper.get('[data-test="task-summary"]').text()).toBe(
      'Runs <i>every</i> check.\nTwice.',
    )
  })

  it('flags an internal task and shows no description it does not have', () => {
    const wrapper = mount(TasksList, { props: { tasks } })
    const second = wrapper.findAll('[data-test="task"]')[1]
    if (second === undefined) throw new Error('no second row')
    expect(second.get('[data-test="task-internal"]').text()).toBe('internal')
    expect(second.get('[data-test="task-desc"]').text()).toBe('')
    expect(second.find('[data-test="task-summary"]').exists()).toBe(false)
    expect(second.find('[data-test="task-aliases"]').exists()).toBe(false)
  })

  it('names the file read and every include it refused', () => {
    const wrapper = mount(TasksList, { props: { tasks } })
    expect(wrapper.get('[data-test="tasks-file"]').text()).toBe('Taskfile.yml')
    const refused = wrapper.findAll('[data-test="task-refused"]')
    expect(refused).toHaveLength(1)
    expect(refused[0]?.text()).toContain('remote')
    expect(refused[0]?.text()).toContain('https://example.invalid/Taskfile.yml')
    expect(refused[0]?.text()).toContain('only files inside the project are read')
  })

  it('a Taskfile without tasks says so', () => {
    const wrapper = mount(TasksList, {
      props: { tasks: { file: 'Taskfile.yml', tasks: [], refused: [], truncated: false } },
    })
    expect(wrapper.findAll('[data-test="task"]')).toHaveLength(0)
    expect(wrapper.get('[data-test="empty"]').text()).toBe('no tasks')
    expect(wrapper.find('[data-test="task-refused"]').exists()).toBe(false)
    expect(wrapper.find('[data-test="tasks-truncated"]').exists()).toBe(false)
  })

  it('a truncated list says it stops at the server limit', () => {
    const wrapper = mount(TasksList, { props: { tasks: { ...tasks, truncated: true } } })
    expect(wrapper.get('[data-test="tasks-truncated"]').text()).toBe(
      'list truncated at 10,000 entries',
    )
  })
})
