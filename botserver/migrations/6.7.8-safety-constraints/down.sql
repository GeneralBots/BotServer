-- Drops only what 6.7.8-safety-constraints created. The earlier
-- `DROP TABLE IF EXISTS safety_constraints CASCADE` in 6.5.23's down.sql refers
-- to a table no migration creates; on a database that predates the consolidated
-- migration that statement would have destroyed rows belonging to an
-- unrecorded table, which is why this migration recreates it explicitly.

DROP INDEX IF EXISTS idx_safety_constraints_bot;

DROP TABLE IF EXISTS safety_constraints;