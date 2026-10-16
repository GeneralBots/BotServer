-- Retail create/edit support tables (issue #1227)
CREATE TABLE IF NOT EXISTS retail_branches (
    id UUID PRIMARY KEY,
    branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    code TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    manager TEXT NOT NULL DEFAULT '',
    stock_value NUMERIC(18,2) NOT NULL DEFAULT 0,
    status VARCHAR(30) NOT NULL DEFAULT 'active',
    pricing_rules JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS retail_promotions (
    id UUID PRIMARY KEY,
    branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    name TEXT NOT NULL DEFAULT '',
    type TEXT NOT NULL DEFAULT '',
    discount TEXT NOT NULL DEFAULT '',
    valid_from DATE,
    valid_to DATE,
    status VARCHAR(30) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS retail_suppliers (
    id UUID PRIMARY KEY,
    branch_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    cnpj TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    contact TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    lead_time_days INTEGER NOT NULL DEFAULT 0,
    rating NUMERIC(3,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
