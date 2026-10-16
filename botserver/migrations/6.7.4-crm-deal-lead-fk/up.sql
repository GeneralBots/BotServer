-- #1441 A3 — `crm_deals.lead_id` is the "converted from this lead" pointer, but
-- migration 6.2.3-01-crm-deals declared it as a foreign key to `crm_leads(id)`.
-- This deployment stores leads in `crm_deals` itself (`crm_leads` is empty), so
-- `POST /api/crm/leads/:id/convert` always failed with
-- `crm_deals_lead_id_fkey` — the Lead → Opportunity flow dead-ended at the last
-- step and the Opportunities grid could never show a converted lead.
--
-- The constraint is repointed at `crm_deals(id)` (a self-reference), which is
-- what the handler actually writes. Legacy pointers that do not resolve inside
-- `crm_deals` are detached first so the migration cannot fail on old data.
UPDATE crm_deals d
   SET lead_id = NULL
 WHERE d.lead_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM crm_deals s WHERE s.id = d.lead_id);

ALTER TABLE crm_deals DROP CONSTRAINT IF EXISTS crm_deals_lead_id_fkey;

-- Guarded so the migration is idempotent: databases that applied this under its
-- earlier version string (2026-10-01-193000) re-run it once after the renumber.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
         WHERE conname = 'crm_deals_lead_id_fkey'
           AND conrelid = 'crm_deals'::regclass
    ) THEN
        ALTER TABLE crm_deals
            ADD CONSTRAINT crm_deals_lead_id_fkey
            FOREIGN KEY (lead_id) REFERENCES crm_deals(id);
    END IF;
END $$;
