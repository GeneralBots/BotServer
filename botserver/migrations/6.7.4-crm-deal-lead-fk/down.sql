-- Restores the 6.2.3 declaration (FK to the legacy `crm_leads` table). Rows
-- created by the CRM since this migration points at `crm_deals`, so rolling
-- back requires detaching them first.
UPDATE crm_deals d
   SET lead_id = NULL
 WHERE d.lead_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM crm_leads l WHERE l.id = d.lead_id);

ALTER TABLE crm_deals DROP CONSTRAINT IF EXISTS crm_deals_lead_id_fkey;

ALTER TABLE crm_deals
    ADD CONSTRAINT crm_deals_lead_id_fkey
    FOREIGN KEY (lead_id) REFERENCES crm_leads(id);
