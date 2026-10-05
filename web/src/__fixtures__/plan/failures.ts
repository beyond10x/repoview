import type { ApiResult } from '../../api/client'

/** An error result whose response carried the JSON `body`, as the server's 502 and 503 do. */
export function failedWith(status: number, body: unknown): ApiResult<never> {
  return Object.assign(
    { state: 'error' as const, status, message: `HTTP ${String(status)}` },
    { body },
  )
}
