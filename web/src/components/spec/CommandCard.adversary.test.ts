import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { billing } from '../../__fixtures__/spec'
import CommandCard from './CommandCard.vue'

// Adversary, story:spec-pages pass 2. ess 0.52.0 writes a `when` outcome's condition as
// `{ "kind": "when", "predicate": "<expression>" }` with no `cause` (billing.compile.json,
// `billing.invoice.CreateInvoice`). The predicate is the condition the outcome is taken on; the
// page must show it rather than the bare kind.

describe('a when outcome', () => {
  it('shows the predicate ess compiled for it', () => {
    const command = billing.ir.commands['billing.invoice.CreateInvoice']
    if (command === undefined) throw new Error('no CreateInvoice in the billing fixture')
    const wrapper = mount(CommandCard, { props: { command, anchors: new Map<string, string>() } })
    const accepted = wrapper
      .findAll('[data-test="outcome"]')
      .find((item) => item.get('.outcome-name').text() === 'accepted')
    if (accepted === undefined) throw new Error('no accepted outcome rendered')
    expect(accepted.text()).toContain('amount.amount > 0')
    expect(accepted.text()).not.toMatch(/when\s+when/)
  })
})
