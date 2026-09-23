import { QueryClient } from '@tanstack/react-query'

export const MAX_RETRIES = 2
export const MAX_RETRY_DELAY_MS = 10_000


export function exponentialBackoffDelayMs(attempt: number): number {
  return Math.min(1000 * 2 ** attempt, MAX_RETRY_DELAY_MS)
}

export function repositoryAnalysisQueryKey(owner: string, repo: string) {
  return ['repositoryAnalysis', owner, repo] as const
}

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 0,
      gcTime: 0,
      retry: MAX_RETRIES,
      retryDelay: (attemptIndex) =>
        exponentialBackoffDelayMs(attemptIndex),
    },
  },
})
