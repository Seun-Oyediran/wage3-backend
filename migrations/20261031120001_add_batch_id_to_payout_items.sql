-- Which on-chain batch each person is paid in. NULL while the payout is a draft;
-- set at prepare.
ALTER TABLE payout_items
    ADD COLUMN IF NOT EXISTS batch_id UUID REFERENCES payout_batches (id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS payout_items_batch_id_idx ON payout_items (batch_id);
