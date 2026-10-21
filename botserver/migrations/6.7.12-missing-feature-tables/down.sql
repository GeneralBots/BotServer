-- Drops only the tables created by 6.7.12-missing-feature-tables.

DROP INDEX IF EXISTS idx_video_keyframes_layer;
DROP TABLE IF EXISTS video_keyframes;

DROP TABLE IF EXISTS warmup_schedules;

DROP INDEX IF EXISTS idx_marketing_email_opens_email;
DROP TABLE IF EXISTS marketing_email_opens;

DROP INDEX IF EXISTS idx_org_ips_org;
DROP TABLE IF EXISTS org_ips;

DROP INDEX IF EXISTS idx_ip_rotations_org;
DROP TABLE IF EXISTS ip_rotations;

DROP TABLE IF EXISTS ip_reputation;

DROP INDEX IF EXISTS idx_campaign_metrics_campaign;
DROP TABLE IF EXISTS campaign_metrics;
