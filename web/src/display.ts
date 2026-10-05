/** True when the server gave text worth showing: not null, not empty, not only whitespace. */
export function hasText(text: string | null | undefined): text is string {
  return text !== null && text !== undefined && text.trim() !== ''
}

/**
 * Every server string reaches the DOM through this, so an empty or missing value renders as the
 * named fallback and never as an empty element.
 */
export function orElse(text: string | null | undefined, fallback: string): string {
  return hasText(text) ? text : fallback
}
