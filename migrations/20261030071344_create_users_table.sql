-- A person who signs in to the app (sender side). Organisations, wallets and
-- recipients live in their own tables; a user joins an organisation as a member.
-- No password: users sign in with Google or an email code, both of which prove
-- they own the email address.
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name TEXT,
    email TEXT NOT NULL,
    google_sub TEXT UNIQUE, -- Google's stable account ID (`sub` claim); NULL until they use Google
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Case-insensitive: Ada@Acme.com and ada@acme.com are the same account.
CREATE UNIQUE INDEX IF NOT EXISTS users_email_key ON users (LOWER(email));
