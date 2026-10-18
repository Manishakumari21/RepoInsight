CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    session_hash TEXT NOT NULL UNIQUE,
    github_user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL
);

CREATE INDEX sessions_active_expiry_idx
    ON sessions (expires_at)
    WHERE revoked_at IS NULL;
