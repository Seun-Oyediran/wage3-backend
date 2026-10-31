-- The people saved in a list. Deleted with their list. Paying a list copies these
-- into payout_items, so deleting or editing them never changes a payout.
CREATE TABLE IF NOT EXISTS recipients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_id UUID NOT NULL REFERENCES lists (id) ON DELETE CASCADE,
    position INT NOT NULL, -- display order
    name TEXT,
    email TEXT, -- lowercase
    wallet_address TEXT NOT NULL, -- lowercase 0x...
    default_amount NUMERIC(78, 0) CHECK (default_amount > 0), -- base units; pre-fills each payout
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS recipients_list_id_idx ON recipients (list_id, position);
