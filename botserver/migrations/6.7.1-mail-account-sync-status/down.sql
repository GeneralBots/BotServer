DROP INDEX IF EXISTS idx_user_email_accounts_sync_health;

ALTER TABLE user_email_accounts
    DROP COLUMN IF EXISTS last_error_at,
    DROP COLUMN IF EXISTS last_error,
    DROP COLUMN IF EXISTS last_sync_at;
