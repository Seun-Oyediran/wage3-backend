-- One-time sign-in codes sent by email. Keyed by email, not user, because the
-- first code is sent before the account exists (sign-up and login are the same flow).
CREATE TABLE IF NOT EXISTS login_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL,
    code_hash TEXT NOT NULL, -- HMAC of the code with a server secret, never the code itself
    attempts INT NOT NULL DEFAULT 0, -- wrong guesses so far; the code is dead after 5
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ, -- set when used; a code works once
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS login_codes_email_idx ON login_codes (LOWER(email), created_at DESC);
