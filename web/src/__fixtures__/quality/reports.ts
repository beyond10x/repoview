// `/api/quality` documents for the Quality page tests: what the server answers with the beyond10x
// codegate 0.3.0 on PATH (story:quality-codegate acceptance 2).

import type { QualityReport } from '../../api/quality'

export const TOOL_PATH = '/opt/bin/codegate'

export const GO_PATH = '/opt/go/bin/codegate'

export const REASON =
  'codegate 0.3.0 evaluates supplied dependency facts only; it has no source assessment yet'

/** codegate 0.3.0 found first on PATH. */
export const codegate030: QualityReport = {
  tool: 'codegate',
  tool_path: TOOL_PATH,
  tool_version: '0.3.0',
  skipped: [],
  commands: ['evaluate'],
  assessment: null,
  reason: REASON,
}

/** The Go codegate first on PATH, skipped, then codegate 0.3.0. */
export const goSkipped: QualityReport = { ...codegate030, skipped: [GO_PATH] }

/**
 * A report carrying score-shaped data it must never show: `assessment` is null, yet the
 * document also holds a rating, scores and findings as the Go adapter used to send them.
 */
export const strayScores = {
  ...codegate030,
  rating: 'B-',
  score_max: 100,
  scores: { overall: 67 },
  top_findings: [{ kind: 'k', reason: 'Document has no H1 title.', location: { uri: 'a.md' } }],
  languages: [
    {
      language: 'markdown',
      status: 'assessed',
      reason: null,
      assessment: { rating: 'B-', scores: { overall: 67 } },
    },
  ],
}
