ALTER TABLE github_tokens
    ADD COLUMN access_token_nonce BYTEA NOT NULL,
    ADD COLUMN refresh_token_nonce BYTEA NULL;

ALTER TABLE github_tokens
    ADD CONSTRAINT github_tokens_access_token_nonce_length
        CHECK (octet_length(access_token_nonce) = 12),
    ADD CONSTRAINT github_tokens_refresh_token_nonce_length
        CHECK (refresh_token_nonce IS NULL OR octet_length(refresh_token_nonce) = 12);
