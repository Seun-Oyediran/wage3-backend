-- One row per on-chain batchTransfer call. A payout is split into batches of up to
-- 200 people at prepare. `id` is the ref passed to the contract (a UUID is exactly
-- 16 bytes = bytes16), so onchain_batches.batch_ref joins straight to it.
CREATE TABLE IF NOT EXISTS payout_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payout_id UUID NOT NULL REFERENCES payouts (id) ON DELETE CASCADE,
    batch_index INT NOT NULL, -- 0, 1, 2... in send order
    total_amount NUMERIC(78, 0) NOT NULL, -- base units
    recipient_count INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ready'
        CHECK (status IN ('ready', 'submitted', 'confirmed', 'dropped')),
    tx_hash TEXT, -- reported by the frontend, then confirmed by the indexer
    block_number BIGINT,
    submitted_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (payout_id, batch_index)
);
