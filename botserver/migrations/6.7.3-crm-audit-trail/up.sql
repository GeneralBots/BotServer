-- CRM audit trail (#1441 P2) — every destructive or state-changing CRM
-- operation (delete, bulk action, stage transition, lead conversion) writes a
-- row here so tenant admins can answer "who changed this record, when, and
-- from what value to what value". Rows are never updated or deleted by the
-- application; retention is a database policy concern.
CREATE TABLE IF NOT EXISTS crm_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL,
    entity VARCHAR(40) NOT NULL,
    entity_id UUID,
    action VARCHAR(40) NOT NULL,
    actor_email VARCHAR(255),
    before JSONB,
    after JSONB,
    detail JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_crm_audit_logs_branch_created
    ON crm_audit_logs (branch_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_audit_logs_entity
    ON crm_audit_logs (entity, entity_id);
