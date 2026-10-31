-- One row per PaymentSent event (one per recipient). Source of truth for what
-- actually moved on-chain; sent/received stats are computed from this table.
CREATE TABLE IF NOT EXISTS onchain_payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id BIGINT NOT NULL,
    batch_ref UUID NOT NULL,
    sender_address TEXT NOT NULL, -- lowercase 0x...
    recipient_address TEXT NOT NULL, -- lowercase 0x...
    token_address TEXT NOT NULL,
    amount NUMERIC(78, 0) NOT NULL, -- uint256 in base units
    tx_hash TEXT NOT NULL,
    log_index INT NOT NULL, -- log order matches the order of the recipients array
    block_number BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, tx_hash, log_index)
);

CREATE INDEX IF NOT EXISTS onchain_payments_ref_idx ON onchain_payments (batch_ref);
CREATE INDEX IF NOT EXISTS onchain_payments_sender_idx ON onchain_payments (sender_address);
CREATE INDEX IF NOT EXISTS onchain_payments_recipient_idx ON onchain_payments (recipient_address);
