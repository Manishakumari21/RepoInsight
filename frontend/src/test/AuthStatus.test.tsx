import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AuthStatus } from '../components/AuthStatus'

describe('AuthStatus', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('shows loading then unauthenticated', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ authenticated: false, github_user_id: null }),
    }) as any
    render(<AuthStatus />)
    expect(screen.getByText(/Loading auth/i)).toBeInTheDocument()
    await waitFor(() => expect(screen.getByRole('button', { name: /Connect GitHub/i })).toBeInTheDocument())
  })

  it('shows authenticated user', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ authenticated: true, github_user_id: '123' }),
    }) as any
    render(<AuthStatus />)
    await waitFor(() => expect(screen.getByText(/GitHub: 123/i)).toBeInTheDocument())
  })

  it('shows error on status failure', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('boom'))
    render(<AuthStatus />)
    await waitFor(() => expect(screen.getByText(/Failed to load authentication status/i)).toBeInTheDocument())
  })

  it('logout success refreshes status', async () => {
    global.fetch = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, json: async () => ({ authenticated: true, github_user_id: '123' }) })
      .mockResolvedValueOnce({ ok: true })
      .mockResolvedValueOnce({ ok: true, json: async () => ({ authenticated: false, github_user_id: null }) })
    render(<AuthStatus />)
    await waitFor(() => expect(screen.getByRole('button', { name: /Disconnect GitHub/i })).toBeInTheDocument())
    fireEvent.click(screen.getByRole('button', { name: /Disconnect GitHub/i }))
    await waitFor(() => expect(screen.getByRole('button', { name: /Connect GitHub/i })).toBeInTheDocument())
  })

  it('logout failure shows error', async () => {
    global.fetch = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, json: async () => ({ authenticated: true, github_user_id: '123' }) })
      .mockResolvedValueOnce({ ok: false, status: 403 })
    render(<AuthStatus />)
    await waitFor(() => expect(screen.getByRole('button', { name: /Disconnect GitHub/i })).toBeInTheDocument())
    fireEvent.click(screen.getByRole('button', { name: /Disconnect GitHub/i }))
    await waitFor(() => expect(screen.getByText(/Logout failed/i)).toBeInTheDocument())
  })
})
