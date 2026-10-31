-- One payment run: "October payroll". Created as a quick payment, from a list,
-- or by paying a past payout again. Its people live in payout_items.
CREATE TABLE IF NOT EXISTS payouts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    list_id UUID REFERENCES lists (id) ON DELETE SET NULL, -- the list it started from, if any
    name TEXT NOT NULL,
    chain_id BIGINT NOT NULL,
    token_address TEXT NOT NULL, -- lowercase 0x...
    sender_address TEXT, -- the wallet paying; set at prepare
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'ready', 'submitted', 'paid', 'partially_paid', 'cancelled')),
    total_amount NUMERIC(78, 0), -- set at prepare, when the items are locked
    recipient_count INT, -- set at prepare
    prepared_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- "My payouts, newest first".
CREATE INDEX IF NOT EXISTS payouts_user_id_idx ON payouts (user_id, created_at DESC);
