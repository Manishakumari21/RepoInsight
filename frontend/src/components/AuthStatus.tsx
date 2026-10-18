import { useEffect, useState } from 'react'
import { fetchAuthStatus, logout } from '../api'
import './AuthStatus.css'

export function AuthStatus() {
  const [status, setStatus] = useState<{ authenticated: boolean; github_user_id: string | null } | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    fetchAuthStatus()
      .then(setStatus)
      .catch(() => setError('Failed to load authentication status'))
      .finally(() => setLoading(false))
  }, [])

  const handleLogin = () => {
    window.location.href = `${import.meta.env.VITE_API_BASE ?? ''}/auth/github`
  }

  const handleLogout = async () => {
    try {
      await logout()
      setStatus({ authenticated: false, github_user_id: null })
    } catch {
      setError('Logout failed')
    }
  }

  if (loading) return <span>Loading auth…</span>
  if (error) return <span>{error}</span>
  if (status?.authenticated) {
    return (
      <div>
        <span>GitHub: {status.github_user_id}</span>
        <button onClick={handleLogout}>Disconnect GitHub</button>
      </div>
    )
  }
  return <button onClick={handleLogin}>Connect GitHub</button>
}
