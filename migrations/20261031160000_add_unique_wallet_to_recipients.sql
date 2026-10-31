-- A wallet appears at most once per list.
CREATE UNIQUE INDEX IF NOT EXISTS recipients_list_wallet_key ON recipients (list_id, wallet_address);
