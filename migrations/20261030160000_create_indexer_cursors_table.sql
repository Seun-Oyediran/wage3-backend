-- Where each indexer is up to. Survives restarts, so indexing continues from
-- the last saved block instead of starting over.
CREATE TABLE IF NOT EXISTS indexer_cursors (
    name TEXT PRIMARY KEY, -- e.g. 'batcher:84532:0x5473...'
    last_block BIGINT NOT NULL, -- last block fully indexed
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
