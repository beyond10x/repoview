// `/api/quality` documents for the Quality page tests. The markdown assessment is the real output
// of `codegate --root . --language markdown --format json assess --gate all` on this repository.

import type { Assessment, QualityReport } from '../../api/quality'
import captured from './assess-markdown.json'

export const markdownAssessment = captured as Assessment

export const TOOL_PATH = '/opt/bin/codegate'

/** One language in each status. */
export const everyStatus: QualityReport = {
  tool: 'codegate',
  tool_path: TOOL_PATH,
  languages: [
    { language: 'go', status: 'running', reason: null, stderr: null, assessment: null },
    {
      language: 'rust',
      status: 'not-assessed',
      reason: 'codegate does not support rust',
      stderr: null,
      assessment: null,
    },
    {
      language: 'java',
      status: 'failed',
      reason: 'codegate assess failed',
      stderr: 'codegate: java backend crashed\npanic: index out of range',
      assessment: null,
    },
    {
      language: 'markdown',
      status: 'assessed',
      reason: null,
      stderr: null,
      assessment: markdownAssessment,
    },
  ],
}

/** Every language settled: the running one has finished as assessed. */
export const settled: QualityReport = {
  ...everyStatus,
  languages: everyStatus.languages.map((entry) =>
    entry.status === 'running'
      ? { ...entry, status: 'assessed', assessment: markdownAssessment }
      : entry,
  ),
}

/**
 * A server that wrongly attaches an assessment to every language that was not assessed. The page
 * must still show no score for them.
 */
export const strayAssessments: QualityReport = {
  ...everyStatus,
  languages: everyStatus.languages.map((entry) => ({ ...entry, assessment: markdownAssessment })),
}
