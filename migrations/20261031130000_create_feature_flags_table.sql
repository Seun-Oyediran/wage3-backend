-- Switches that can be flipped without a deploy. 'payments' is the emergency breaker:
-- when disabled, no payout can be prepared, so no new payment can start from the app.
-- It does not stop the contract itself; anyone can still call it directly.
CREATE TABLE IF NOT EXISTS feature_flags (
    key TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL,
    reason TEXT, -- shown to users while disabled
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO feature_flags (key, enabled)
VALUES ('payments', TRUE)
ON CONFLICT (key) DO NOTHING;
