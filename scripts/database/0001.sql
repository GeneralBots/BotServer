CREATE TABLE clicks (
    campaign_id TEXT NOT NULL,
    email TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(campaign_id, email)
);