-- `users.is_admin` exists in the code and in `6.5.15.1-consolidated`, but not in
-- the database.
--
-- `6.0.0-01-core` creates `users` first, without `is_admin`; the consolidated
-- migration repeats `CREATE TABLE IF NOT EXISTS users (... is_admin ...)`, which
-- is a no-op because the table already exists. So a database built from
-- `botserver/migrations` never had the column, and every `users.is_admin`
-- reference (provisioning INSERT, directory api SELECT, the admin/user role in
-- the directory UI) failed at runtime.
--
-- Added last so the physical column order of the existing columns is unchanged.
-- Nullable with a default: pre-existing rows have no meaningful admin flag, and
-- the RBAC tables (`rbac_user_groups`) remain the source of truth for
-- authorization; this column is the legacy directory view of it.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
