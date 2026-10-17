-- `6.5.15.1-consolidated` creates both flags as NOT NULL, but the incremental
-- migrations that introduced them (`6.3.2-01-add-bots-public` and
-- `6.5.15-org-branches`) left them nullable, so databases built by the
-- incremental path drifted from a fresh install. Backfill and enforce NOT NULL
-- so every declaration can use `Bool` instead of `Nullable<Bool>`.
UPDATE bots SET is_public = FALSE WHERE is_public IS NULL;
ALTER TABLE bots ALTER COLUMN is_public SET DEFAULT FALSE;
ALTER TABLE bots ALTER COLUMN is_public SET NOT NULL;

UPDATE bots SET is_default_for_branch = FALSE WHERE is_default_for_branch IS NULL;
ALTER TABLE bots ALTER COLUMN is_default_for_branch SET DEFAULT FALSE;
ALTER TABLE bots ALTER COLUMN is_default_for_branch SET NOT NULL;
