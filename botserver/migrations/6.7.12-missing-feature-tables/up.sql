-- Tables declared in `botschema` and queried by the marketing, security and
-- video crates, but never created by any migration. A database built from
-- `botserver/migrations` therefore lacked them, and the affected features
-- failed at runtime with "relation does not exist".
--
-- Shapes follow the canonical declarations in `botschema` (people/part3.rs for
-- the marketing/IP tables, video.rs for `video_keyframes`).

-- Marketing attribution metrics (botmarketing::advisor).
CREATE TABLE IF NOT EXISTS campaign_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL,
    campaign_id UUID,
    metric_type VARCHAR(50) NOT NULL,
    metric_value NUMERIC NOT NULL DEFAULT 0,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_campaign_metrics_campaign
    ON campaign_metrics (campaign_id, metric_type);

-- Per-IP sender reputation (botmarketing::ip_router).
CREATE TABLE IF NOT EXISTS ip_reputation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    ip VARCHAR(64) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    delivered BIGINT NOT NULL DEFAULT 0,
    bounced BIGINT NOT NULL DEFAULT 0,
    complained BIGINT NOT NULL DEFAULT 0,
    window_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    UNIQUE (org_id, ip, provider)
);

-- Outbound IP rotation state (botmarketing::ip_router).
CREATE TABLE IF NOT EXISTS ip_rotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    ip_address VARCHAR(64) NOT NULL,
    strategy VARCHAR(50),
    current_index INTEGER,
    last_used TIMESTAMPTZ,
    is_active BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ip_rotations_org
    ON ip_rotations (org_id, is_active);

-- IP pool per organization (botmarketing::ip_router).
CREATE TABLE IF NOT EXISTS org_ips (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    ip_address VARCHAR(64) NOT NULL,
    provider VARCHAR(100),
    is_active BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_org_ips_org
    ON org_ips (org_id, is_active);

-- Email-open tracking used by the warm-up campaign (botmarketing::warmup).
CREATE TABLE IF NOT EXISTS marketing_email_opens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL,
    campaign_id UUID,
    email VARCHAR(255) NOT NULL,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(64),
    user_agent TEXT
);

CREATE INDEX IF NOT EXISTS idx_marketing_email_opens_email
    ON marketing_email_opens (email, opened_at);

-- Mailbox warm-up schedules (botmarketing::warmup).
CREATE TABLE IF NOT EXISTS warmup_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL,
    org_id UUID NOT NULL,
    email VARCHAR(255) NOT NULL,
    ip VARCHAR(64) NOT NULL,
    daily_limit INTEGER,
    current_count INTEGER,
    current_day INTEGER,
    started_at TIMESTAMPTZ,
    status VARCHAR(50),
    paused_reason TEXT,
    is_active BOOLEAN,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, email, ip)
);

-- Animation keyframes (botvideo).
CREATE TABLE IF NOT EXISTS video_keyframes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    layer_id UUID NOT NULL REFERENCES video_layers(id) ON DELETE CASCADE,
    property_name TEXT NOT NULL,
    time_ms BIGINT NOT NULL,
    value_json JSONB NOT NULL,
    easing TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_video_keyframes_layer
    ON video_keyframes (layer_id, time_ms);
