-- Safety constraints for the AutoTask intent compiler.
--
-- `SafetyLayer::load_constraints` selects from this table
-- (`SELECT id, name, constraint_type, description, expression, threshold,
-- severity, enabled, applies_to FROM safety_constraints WHERE bot_id = $1 AND
-- enabled = true`), but no migration ever created it: `6.5.23-branch-scope-cleanup`
-- only *guards* its `ADD COLUMN branch_id` behind an existence check, and its
-- down.sql drops the table. A database built from `botserver/migrations`
-- therefore never had it, and the loader swallowed the "relation does not exist"
-- error via `unwrap_or_default()`, so the layer silently ran with no constraints.
--
-- Created with exactly the columns the loader reads. The earlier Rust
-- declaration also carried `pattern`, `action` and `is_active`, which nothing
-- reads and which duplicate `expression`/`threshold` and `enabled` respectively;
-- they are not reproduced here.

CREATE TABLE IF NOT EXISTS safety_constraints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    bot_id UUID NOT NULL,
    name TEXT NOT NULL,
    constraint_type VARCHAR(50) NOT NULL,
    description TEXT,
    expression TEXT,
    threshold TEXT,
    severity TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    applies_to TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The loader filters on bot_id and enabled for every evaluation.
CREATE INDEX IF NOT EXISTS idx_safety_constraints_bot
    ON safety_constraints (bot_id, enabled);