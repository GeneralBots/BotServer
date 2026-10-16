-- Per-account delivery health for the unified inbox.
--
-- A mailbox that cannot be reached was indistinguishable from an empty one:
-- the only signal was a warning in the server log, so users saw an empty inbox
-- with no explanation. These columns record the outcome of the last sync pass
-- so the Mail application can report it next to the account.
ALTER TABLE user_email_accounts
    ADD COLUMN IF NOT EXISTS last_sync_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_error TEXT,
    ADD COLUMN IF NOT EXISTS last_error_at TIMESTAMPTZ;

-- The sync worker filters on is_active; pairing it with last_error_at keeps the
-- health lookup for the account list cheap.
CREATE INDEX IF NOT EXISTS idx_user_email_accounts_sync_health
    ON user_email_accounts (is_active, last_error_at DESC);
