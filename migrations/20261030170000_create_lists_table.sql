-- A reusable list of people a user pays, like a contact list ("Engineering team").
-- Each payout copies the list's recipients, so editing a list never changes past payouts.
CREATE TABLE IF NOT EXISTS lists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- "My lists, newest first" and the base for name search.
CREATE INDEX IF NOT EXISTS lists_user_id_idx ON lists (user_id, created_at DESC);
