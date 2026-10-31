-- One row per "Sign in with Google" attempt, between /auth/google and the callback.
-- `state` is the random value sent to Google and checked when it redirects back
-- (stops CSRF). The row is deleted on first use and is useless after 10 minutes.
CREATE TABLE IF NOT EXISTS oauth_states (
    state TEXT PRIMARY KEY,
    pkce_verifier TEXT NOT NULL, -- proves the code exchange comes from whoever started the flow
    nonce TEXT NOT NULL, -- must match the `nonce` claim in Google's ID token (stops token replay)
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
