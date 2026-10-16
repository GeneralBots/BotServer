DROP INDEX IF EXISTS idx_user_email_accounts_auth_mode;

ALTER TABLE user_email_accounts
    DROP CONSTRAINT IF EXISTS user_email_accounts_auth_mode_check;

ALTER TABLE user_email_accounts
    DROP COLUMN IF EXISTS token_expires_at,
    DROP COLUMN IF EXISTS access_token_encrypted,
    DROP COLUMN IF EXISTS refresh_token_encrypted,
    DROP COLUMN IF EXISTS oauth_provider,
    DROP COLUMN IF EXISTS auth_mode;
