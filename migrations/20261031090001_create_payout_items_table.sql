-- The people in one payout. Copied in (from a list, a past payout, or typed),
-- never linked, so editing a list never changes what a payout paid.
CREATE TABLE IF NOT EXISTS payout_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payout_id UUID NOT NULL REFERENCES payouts (id) ON DELETE CASCADE,
    position INT NOT NULL, -- order on screen and in the on-chain recipients array
    name TEXT,
    email TEXT, -- lowercase
    wallet_address TEXT NOT NULL, -- lowercase 0x...
    amount NUMERIC(78, 0) NOT NULL CHECK (amount > 0), -- base units: 1 USDC = 1000000
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (payout_id, position)
);
