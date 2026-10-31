-- One row per BatchPayment event (one per batchTransfer call). Chain facts only:
-- linked to app batches later by `batch_ref`, so there are no foreign keys here.
CREATE TABLE IF NOT EXISTS onchain_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id BIGINT NOT NULL,
    batch_ref UUID NOT NULL, -- the contract's bytes16 ref; a UUID is exactly 16 bytes
    sender_address TEXT NOT NULL, -- lowercase 0x...
    token_address TEXT NOT NULL,
    total_amount NUMERIC(78, 0) NOT NULL, -- uint256 in base units; any token can be sent, so not BIGINT
    recipient_count INT NOT NULL,
    tx_hash TEXT NOT NULL,
    log_index INT NOT NULL,
    block_number BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, tx_hash, log_index) -- re-indexing a range is a no-op
);

CREATE INDEX IF NOT EXISTS onchain_batches_ref_idx ON onchain_batches (batch_ref);
CREATE INDEX IF NOT EXISTS onchain_batches_sender_idx ON onchain_batches (sender_address);
