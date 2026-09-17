const API_BASE: string = import.meta.env.VITE_API_BASE ?? ''
export const ANALYSIS_ENDPOINT = `${API_BASE}/api/repositories/{owner}/{repo}/analysis`