import { describe, expect, it, vi } from 'vitest'
import {
  fetchLocalAnalysis,
  fetchLocalPredictions,
  fetchLocalRipple,
  fetchLocalImpact,
} from '../api'

describe('local API production guard', () => {
  it('rejects all local functions in production', async () => {
    vi.stubEnv('PROD', true as any)
    await expect(fetchLocalAnalysis('/tmp/repo')).rejects.toThrow(/local development/i)
    await expect(fetchLocalPredictions('/tmp/repo')).rejects.toThrow(/local development/i)
    await expect(fetchLocalRipple('/tmp/repo', {} as any)).rejects.toThrow(/local development/i)
    await expect(fetchLocalImpact('/tmp/repo', {} as any)).rejects.toThrow(/local development/i)
    vi.unstubAllEnvs()
  })
})