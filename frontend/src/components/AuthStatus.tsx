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

  if (loading) return <span className="auth-status">Loading GitHub connection…</span>
  if (error) return <span className="auth-status auth-error">{error}</span>
  if (status?.authenticated) {
    return (
      <div className="auth-status">
        <span className="auth-user">GitHub connected: {status.github_user_id}</span>
        <button type="button" className="auth-button" onClick={handleLogout}>
          Disconnect GitHub
        </button>
      </div>
    )
  }
  return (
    <button type="button" className="auth-button auth-button-primary" onClick={handleLogin}>
      Connect GitHub
    </button>
  )
}
