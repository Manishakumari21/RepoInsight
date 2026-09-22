import { describe, expect, it } from 'vitest'
import { analysisErrorHint } from '../App'

describe('analysisErrorHint', () => {
  it('explains a dead backend for gateway errors', () => {
    for (const status of [502, 503, 504]) {
      const hint = analysisErrorHint(status)
      expect(hint).toMatch(/unreachable/i)
      expect(hint).toMatch(/3000/)
    }
  })

  it('explains rate limiting and missing repositories', () => {
    expect(analysisErrorHint(429)).toMatch(/rate limit/i)
    expect(analysisErrorHint(404)).toMatch(/not found/i)
  })

  it('falls back to the raw status for unknown codes', () => {
    expect(analysisErrorHint(418)).toMatch(/418/)
  })
})
