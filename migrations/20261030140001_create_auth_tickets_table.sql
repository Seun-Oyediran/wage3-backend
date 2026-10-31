-- One-time tickets handed to the frontend after Google sign-in. The callback
-- redirects to Next.js with the ticket in the URL; Next.js swaps it for real
-- tokens via POST /auth/google/exchange. Tokens never appear in a URL.
CREATE TABLE IF NOT EXISTS auth_tickets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_hash TEXT NOT NULL UNIQUE, -- HMAC of the ticket, never the ticket itself
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL, -- short: about a minute
    consumed_at TIMESTAMPTZ, -- set when exchanged; a ticket works once
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
