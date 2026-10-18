CREATE TABLE github_tokens (
    session_id UUID PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    encrypted_access_token BYTEA NOT NULL,
    encrypted_refresh_token BYTEA NULL,
    access_token_expires_at TIMESTAMPTZ NULL,
    refresh_token_expires_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
