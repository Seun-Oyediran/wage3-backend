-- The token is whatever ERC-20 the user picks (pasted address in the future), so its
-- symbol and decimals are read from the token contract when the payout is created and
-- stored here. History then shows "250 USDC" without asking the chain again.
ALTER TABLE payouts
    ADD COLUMN IF NOT EXISTS token_symbol TEXT,
    ADD COLUMN IF NOT EXISTS token_decimals INT;
