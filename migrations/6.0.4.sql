-- Migration 6.0.4: Configuration Management System
-- Eliminates .env dependency by storing all configuration in database

-- ============================================================================
-- SERVER CONFIGURATION TABLE
-- Stores server-wide configuration (replaces .env variables)
-- ============================================================================
CREATE TABLE IF NOT EXISTS server_configuration (
    id TEXT PRIMARY KEY,
    config_key TEXT NOT NULL UNIQUE,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL DEFAULT 'string', -- string, integer, boolean, encrypted
    description TEXT,
    is_encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_server_config_key ON server_configuration(config_key);
CREATE INDEX IF NOT EXISTS idx_server_config_type ON server_configuration(config_type);

-- ============================================================================
-- TENANT CONFIGURATION TABLE
-- Stores tenant-level configuration (multi-tenancy support)
-- ============================================================================
CREATE TABLE IF NOT EXISTS tenant_configuration (
    id TEXT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL DEFAULT 'string',
    is_encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, config_key)
);

CREATE INDEX IF NOT EXISTS idx_tenant_config_tenant ON tenant_configuration(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_config_key ON tenant_configuration(config_key);

-- ============================================================================
-- BOT CONFIGURATION TABLE
-- Stores bot-specific configuration (replaces bot config JSON)
-- ============================================================================
CREATE TABLE IF NOT EXISTS bot_configuration (
    id TEXT PRIMARY KEY,
    bot_id UUID NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    config_type TEXT NOT NULL DEFAULT 'string',
    is_encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, config_key)
);

CREATE INDEX IF NOT EXISTS idx_bot_config_bot ON bot_configuration(bot_id);
CREATE INDEX IF NOT EXISTS idx_bot_config_key ON bot_configuration(config_key);

-- ============================================================================
-- MODEL CONFIGURATIONS TABLE
-- Stores LLM and Embedding model configurations
-- ============================================================================
CREATE TABLE IF NOT EXISTS model_configurations (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL UNIQUE, -- Friendly name: "deepseek-1.5b", "gpt-oss-20b"
    model_type TEXT NOT NULL, -- 'llm' or 'embed'
    provider TEXT NOT NULL, -- 'openai', 'groq', 'local', 'ollama', etc.
    endpoint TEXT NOT NULL,
    api_key TEXT, -- Encrypted
    model_id TEXT NOT NULL, -- Actual model identifier
    context_window INTEGER,
    max_tokens INTEGER,
    temperature REAL DEFAULT 0.7,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_default BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_model_config_type ON model_configurations(model_type);
CREATE INDEX IF NOT EXISTS idx_model_config_active ON model_configurations(is_active);
CREATE INDEX IF NOT EXISTS idx_model_config_default ON model_configurations(is_default);

-- ============================================================================
-- CONNECTION CONFIGURATIONS TABLE
-- Stores custom database connections (replaces CUSTOM_* env vars)
-- ============================================================================
CREATE TABLE IF NOT EXISTS connection_configurations (
    id TEXT PRIMARY KEY,
    bot_id UUID NOT NULL,
    connection_name TEXT NOT NULL, -- Used in BASIC: FIND "conn1.table"
    connection_type TEXT NOT NULL, -- 'postgres', 'mysql', 'mssql', 'mongodb', etc.
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    database_name TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL, -- Encrypted
    ssl_enabled BOOLEAN NOT NULL DEFAULT false,
    additional_params JSONB DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, connection_name)
);

CREATE INDEX IF NOT EXISTS idx_connection_config_bot ON connection_configurations(bot_id);
CREATE INDEX IF NOT EXISTS idx_connection_config_name ON connection_configurations(connection_name);
CREATE INDEX IF NOT EXISTS idx_connection_config_active ON connection_configurations(is_active);

-- ============================================================================
-- COMPONENT INSTALLATIONS TABLE
-- Tracks installed components (postgres, minio, qdrant, etc.)
-- ============================================================================
CREATE TABLE IF NOT EXISTS component_installations (
    id TEXT PRIMARY KEY,
    component_name TEXT NOT NULL UNIQUE, -- 'tables', 'drive', 'vectordb', 'cache', 'llm'
    component_type TEXT NOT NULL, -- 'database', 'storage', 'vector', 'cache', 'compute'
    version TEXT NOT NULL,
    install_path TEXT NOT NULL, -- Relative to botserver-stack
    binary_path TEXT, -- Path to executable
    data_path TEXT, -- Path to data directory
    config_path TEXT, -- Path to config file
    log_path TEXT, -- Path to log directory
    status TEXT NOT NULL DEFAULT 'stopped', -- 'running', 'stopped', 'error', 'installing'
    port INTEGER,
    pid INTEGER,
    auto_start BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb,
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_started_at TIMESTAMPTZ,
    last_stopped_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_component_name ON component_installations(component_name);
CREATE INDEX IF NOT EXISTS idx_component_status ON component_installations(status);

-- ============================================================================
-- TENANTS TABLE
-- Multi-tenancy support
-- ============================================================================
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_active ON tenants(is_active);

-- ============================================================================
-- BOT SESSIONS ENHANCEMENT
-- Add tenant_id to existing sessions if column doesn't exist
-- ============================================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'user_sessions' AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE user_sessions ADD COLUMN tenant_id UUID;
        CREATE INDEX idx_user_sessions_tenant ON user_sessions(tenant_id);
    END IF;
END $$;

-- ============================================================================
-- BOTS TABLE ENHANCEMENT
-- Add tenant_id if it doesn't exist
-- ============================================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'bots' AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE bots ADD COLUMN tenant_id UUID;
        CREATE INDEX idx_bots_tenant ON bots(tenant_id);
    END IF;
END $$;

-- ============================================================================
-- DEFAULT SERVER CONFIGURATION
-- Insert default values that replace .env
-- ============================================================================
INSERT INTO server_configuration (id, config_key, config_value, config_type, description) VALUES
    (gen_random_uuid()::text, 'SERVER_HOST', '127.0.0.1', 'string', 'Server bind address'),
    (gen_random_uuid()::text, 'SERVER_PORT', '8080', 'integer', 'Server port'),
    (gen_random_uuid()::text, 'TABLES_SERVER', 'localhost', 'string', 'PostgreSQL server address'),
    (gen_random_uuid()::text, 'TABLES_PORT', '5432', 'integer', 'PostgreSQL port'),
    (gen_random_uuid()::text, 'TABLES_DATABASE', 'botserver', 'string', 'PostgreSQL database name'),
    (gen_random_uuid()::text, 'TABLES_USERNAME', 'botserver', 'string', 'PostgreSQL username'),
    (gen_random_uuid()::text, 'DRIVE_SERVER', 'localhost:9000', 'string', 'MinIO server address'),
    (gen_random_uuid()::text, 'DRIVE_USE_SSL', 'false', 'boolean', 'Use SSL for drive'),
    (gen_random_uuid()::text, 'DRIVE_ORG_PREFIX', 'botserver', 'string', 'Drive organization prefix'),
    (gen_random_uuid()::text, 'DRIVE_BUCKET', 'default', 'string', 'Default S3 bucket'),
    (gen_random_uuid()::text, 'VECTORDB_URL', 'http://localhost:6333', 'string', 'Qdrant vector database URL'),
    (gen_random_uuid()::text, 'CACHE_URL', 'redis://localhost:6379', 'string', 'Redis cache URL'),
    (gen_random_uuid()::text, 'STACK_PATH', './botserver-stack', 'string', 'Base path for all components'),
    (gen_random_uuid()::text, 'SITES_ROOT', './botserver-stack/sites', 'string', 'Root path for sites')
ON CONFLICT (config_key) DO NOTHING;

-- ============================================================================
-- DEFAULT TENANT
-- Create default tenant for single-tenant installations
-- ============================================================================
INSERT INTO tenants (id, name, slug, is_active) VALUES
    (gen_random_uuid(), 'Default Tenant', 'default', true)
ON CONFLICT (slug) DO NOTHING;

-- ============================================================================
-- DEFAULT MODELS
-- Add some default model configurations
-- ============================================================================
INSERT INTO model_configurations (id, model_name, model_type, provider, endpoint, model_id, context_window, max_tokens, is_default) VALUES
    (gen_random_uuid()::text, 'gpt-4', 'llm', 'openai', 'https://api.openai.com/v1', 'gpt-4', 8192, 4096, true),
    (gen_random_uuid()::text, 'gpt-3.5-turbo', 'llm', 'openai', 'https://api.openai.com/v1', 'gpt-3.5-turbo', 4096, 2048, false),
    (gen_random_uuid()::text, 'bge-large', 'embed', 'local', 'http://localhost:8081', 'BAAI/bge-large-en-v1.5', 512, 1024, true)
ON CONFLICT (model_name) DO NOTHING;

-- ============================================================================
-- COMPONENT LOGGING TABLE
-- Track component lifecycle events
-- ============================================================================
CREATE TABLE IF NOT EXISTS component_logs (
    id TEXT PRIMARY KEY,
    component_name TEXT NOT NULL,
    log_level TEXT NOT NULL, -- 'info', 'warning', 'error', 'debug'
    message TEXT NOT NULL,
    details JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_component_logs_component ON component_logs(component_name);
CREATE INDEX IF NOT EXISTS idx_component_logs_level ON component_logs(log_level);
CREATE INDEX IF NOT EXISTS idx_component_logs_created ON component_logs(created_at);

-- ============================================================================
-- GBOT CONFIG SYNC TABLE
-- Tracks .gbot/config.csv file changes and last sync
-- ============================================================================
CREATE TABLE IF NOT EXISTS gbot_config_sync (
    id TEXT PRIMARY KEY,
    bot_id UUID NOT NULL UNIQUE,
    config_file_path TEXT NOT NULL,
    last_sync_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    file_hash TEXT NOT NULL,
    sync_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_gbot_sync_bot ON gbot_config_sync(bot_id);

-- ============================================================================
-- VIEWS FOR EASY QUERYING
-- ============================================================================

-- View: All active components
CREATE OR REPLACE VIEW v_active_components AS
SELECT
    component_name,
    component_type,
    version,
    status,
    port,
    installed_at,
    last_started_at
FROM component_installations
WHERE status = 'running'
ORDER BY component_name;

-- View: Bot with all configurations
CREATE OR REPLACE VIEW v_bot_full_config AS
SELECT
    b.bot_id,
    b.name as bot_name,
    b.status,
    t.name as tenant_name,
    t.slug as tenant_slug,
    bc.config_key,
    bc.config_value,
    bc.config_type,
    bc.is_encrypted
FROM bots b
LEFT JOIN tenants t ON b.tenant_id = t.id
LEFT JOIN bot_configuration bc ON b.bot_id = bc.bot_id
ORDER BY b.bot_id, bc.config_key;

-- View: Active models by type
CREATE OR REPLACE VIEW v_active_models AS
SELECT
    model_name,
    model_type,
    provider,
    endpoint,
    is_default,
    context_window,
    max_tokens
FROM model_configurations
WHERE is_active = true
ORDER BY model_type, is_default DESC, model_name;

-- ============================================================================
-- FUNCTIONS
-- ============================================================================

-- Function to get configuration value with fallback
CREATE OR REPLACE FUNCTION get_config(
    p_key TEXT,
    p_fallback TEXT DEFAULT NULL
) RETURNS TEXT AS $$
DECLARE
    v_value TEXT;
BEGIN
    SELECT config_value INTO v_value
    FROM server_configuration
    WHERE config_key = p_key;

    RETURN COALESCE(v_value, p_fallback);
END;
$$ LANGUAGE plpgsql;

-- Function to set configuration value
CREATE OR REPLACE FUNCTION set_config(
    p_key TEXT,
    p_value TEXT,
    p_type TEXT DEFAULT 'string',
    p_encrypted BOOLEAN DEFAULT false
) RETURNS VOID AS $$
BEGIN
    INSERT INTO server_configuration (id, config_key, config_value, config_type, is_encrypted, updated_at)
    VALUES (gen_random_uuid()::text, p_key, p_value, p_type, p_encrypted, NOW())
    ON CONFLICT (config_key)
    DO UPDATE SET
        config_value = EXCLUDED.config_value,
        config_type = EXCLUDED.config_type,
        is_encrypted = EXCLUDED.is_encrypted,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_server_config_updated_at BEFORE UPDATE ON server_configuration
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_tenant_config_updated_at BEFORE UPDATE ON tenant_configuration
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_bot_config_updated_at BEFORE UPDATE ON bot_configuration
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_model_config_updated_at BEFORE UPDATE ON model_configurations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_connection_config_updated_at BEFORE UPDATE ON connection_configurations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE server_configuration IS 'Server-wide configuration replacing .env variables';
COMMENT ON TABLE tenant_configuration IS 'Tenant-level configuration for multi-tenancy';
COMMENT ON TABLE bot_configuration IS 'Bot-specific configuration';
COMMENT ON TABLE model_configurations IS 'LLM and embedding model configurations';
COMMENT ON TABLE connection_configurations IS 'Custom database connections for bots';
COMMENT ON TABLE component_installations IS 'Installed component tracking and management';
COMMENT ON TABLE tenants IS 'Tenant management for multi-tenancy';
COMMENT ON TABLE component_logs IS 'Component lifecycle and operation logs';
COMMENT ON TABLE gbot_config_sync IS 'Tracks .gbot/config.csv file synchronization';

-- Migration complete
