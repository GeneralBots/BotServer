-- Restores the DECIMAL(15,2) revenue column from 6.0.7-01-people. The cast
-- rounds, so values may differ by a fraction of a cent.
ALTER TABLE crm_accounts
    ALTER COLUMN annual_revenue TYPE numeric(15,2)
    USING annual_revenue::numeric(15,2);
