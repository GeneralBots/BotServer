-- #1441 C4 — `crm_accounts.annual_revenue` must match the diesel models.
--
-- `6.0.7-01-people` created the column as DECIMAL(15,2) while the consolidated
-- schema (`6.5.15.1-consolidated`) and both diesel models that read it
-- (`botcontacts`, `botemail`) declare DOUBLE PRECISION — the same drift the
-- pipeline went through, which is why `crm_leads_compat` /
-- `crm_opportunities_compat` views exist for the other money columns.
--
-- Effect before this migration: any account row with a revenue failed to
-- deserialize ("Received more than 8 bytes while decoding an f64"), so
-- `GET /api/crm/accounts` returned 500 and the Accounts grid rendered empty.
-- `crm_deals.value` is already double precision, so the account revenue is cast
-- to match it.
--
-- `marketing_campaigns.budget` is deliberately NOT touched: `botmarketing`
-- models it as NUMERIC/BigDecimal (the correct type for money) while
-- `botcontacts` reads it as float8 — the CRM grid therefore casts in SQL
-- (`budget::float8`) instead of changing the column for everyone.
ALTER TABLE crm_accounts
    ALTER COLUMN annual_revenue TYPE double precision
    USING annual_revenue::double precision;
