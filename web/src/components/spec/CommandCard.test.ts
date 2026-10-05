import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { billing, codegate } from '../../__fixtures__/spec'
import type { CommandDecl } from '../../api/spec'
import CommandCard from './CommandCard.vue'

// story:spec-pages, correction round 2: each outcome reads as ess compiled it, from the captured
// billing and codegate IR.

function command(ir: typeof billing.ir, name: string): CommandDecl {
  const found = ir.commands[name]
  if (found === undefined) throw new Error(`no ${name} in the fixture`)
  return found
}

function outcomeText(decl: CommandDecl, name: string): string {
  const wrapper = mount(CommandCard, {
    props: { command: decl, anchors: new Map<string, string>() },
  })
  const item = wrapper
    .findAll('[data-test="outcome"]')
    .find((li) => li.get('.outcome-name').text() === name)
  if (item === undefined) throw new Error(`no outcome ${name}`)
  return item.text().replace(/\s+/g, ' ')
}

describe('outcome conditions, one per kind ess writes', () => {
  it('when: the predicate', () => {
    const text = outcomeText(command(billing.ir, 'billing.invoice.PayInvoice'), 'settled')
    expect(text).toContain('when amount.amount > 0')
    expect(text).not.toMatch(/when\s+when/)
  })

  it('external: the cause', () => {
    const text = outcomeText(command(billing.ir, 'billing.email.SendEmail'), 'failed')
    expect(text).toContain('when the provider rejects the recipient address')
  })

  it('otherwise: "otherwise", never "when otherwise"', () => {
    const text = outcomeText(command(billing.ir, 'billing.email.SendEmail'), 'sent')
    expect(text).toContain('otherwise')
    expect(text).not.toContain('when otherwise')
  })

  it('wrong_state: in words', () => {
    const text = outcomeText(command(billing.ir, 'billing.invoice.CancelInvoice'), 'wrong-state')
    expect(text).toContain('when the entity is in the wrong state')
  })
})

describe('what an outcome does', () => {
  it('names the instance it moves: the supplied identity field', () => {
    const text = outcomeText(command(billing.ir, 'billing.invoice.CancelInvoice'), 'cancelled')
    expect(text).toContain('moves Invoice')
    expect(text).toContain('the supplied invoice_id')
  })

  it('names the instance it creates: observed from the event', () => {
    const text = outcomeText(command(billing.ir, 'billing.invoice.CreateInvoice'), 'accepted')
    expect(text).toContain('creates Invoice')
    expect(text).toContain('observed from InvoiceCreated.invoice_id')
  })

  it('lists what it sets, from an input field or a literal', () => {
    const text = outcomeText(command(billing.ir, 'billing.invoice.CreateInvoice'), 'accepted')
    expect(text).toContain('account_id ← account_id')
    expect(text).toContain('total ← amount')
    expect(text).toContain('reminder_count ← 0')
  })

  it('a command with a response shows its fields, and the outcome that returns it says so', () => {
    const evaluate = command(codegate.ir, 'codegate.dependency.Evaluate')
    const wrapper = mount(CommandCard, {
      props: { command: evaluate, anchors: new Map<string, string>() },
    })
    const response = wrapper.get('[data-test="response"]')
    expect(response.findAll('[data-test="field"]').map((r) => r.attributes('data-field'))).toEqual([
      'report',
    ])
    const outcome = evaluate.outcomes[0]?.name ?? ''
    expect(outcomeText(evaluate, outcome)).toContain('returns the response')
  })
})
