-- Optimized database schema

-- Core tables
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    phone_number VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE IF NOT EXISTS bots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    llm_provider VARCHAR(100) NOT NULL,
    llm_config JSONB NOT NULL DEFAULT '{}',
    context_provider VARCHAR(100) NOT NULL,
    context_config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL DEFAULT 'New Conversation',
    answer_mode INTEGER NOT NULL DEFAULT 0, -- 0=direct, 1=tool
    context_data JSONB NOT NULL DEFAULT '{}',
    current_tool VARCHAR(255),
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, bot_id, title)
);

CREATE TABLE IF NOT EXISTS message_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES user_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role INTEGER NOT NULL, -- 0=user, 1=assistant, 2=system
    content_encrypted TEXT NOT NULL,
    message_type INTEGER NOT NULL DEFAULT 0, -- 0=text, 1=image, 2=audio, 3=file
    media_url TEXT,
    token_count INTEGER NOT NULL DEFAULT 0,
    processing_time_ms INTEGER,
    llm_model VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message_index INTEGER NOT NULL
);

-- Channel and integration tables
CREATE TABLE IF NOT EXISTS bot_channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    channel_type INTEGER NOT NULL, -- 0=web, 1=whatsapp, 2=voice, 3=api
    config JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, channel_type)
);

CREATE TABLE IF NOT EXISTS whatsapp_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    phone_number VARCHAR(50) NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(phone_number, bot_id)
);

-- Automation tables
CREATE TABLE IF NOT EXISTS system_automations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind INTEGER NOT NULL, -- 0=scheduled, 1=table_update, 2=table_insert, 3=table_delete
    target VARCHAR(32),
    schedule CHAR(12),
    param VARCHAR(32) NOT NULL,
    is_active BOOLEAN DEFAULT true NOT NULL,
    last_triggered TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS clicks (
    campaign_id TEXT NOT NULL,
    email TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(campaign_id, email)
);

-- Tools and context tables
CREATE TABLE IF NOT EXISTS tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    script TEXT NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS context_injections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES user_sessions(id) ON DELETE CASCADE,
    injected_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    context_data JSONB NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Analytics tables
CREATE TABLE IF NOT EXISTS usage_analytics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES user_sessions(id) ON DELETE CASCADE,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    total_processing_time_ms INTEGER NOT NULL DEFAULT 0
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_user_sessions_user_bot ON user_sessions(user_id, bot_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_updated_at ON user_sessions(updated_at);
CREATE INDEX IF NOT EXISTS idx_message_history_session_id ON message_history(session_id);
CREATE INDEX IF NOT EXISTS idx_message_history_created_at ON message_history(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_analytics_date ON usage_analytics(date);
CREATE INDEX IF NOT EXISTS idx_system_automations_active ON system_automations(kind) WHERE is_active;
CREATE INDEX IF NOT EXISTS idx_bot_channels_type ON bot_channels(channel_type) WHERE is_active;

-- Default data
INSERT INTO bots (id, name, llm_provider, context_provider)
VALUES ('00000000-0000-0000-0000-000000000000', 'Default Bot', 'mock', 'mock')
ON CONFLICT (id) DO NOTHING;

INSERT INTO users (id, username, email, password_hash)
VALUES ('00000000-0000-0000-0000-000000000001', 'demo', 'demo@example.com', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG')
ON CONFLICT (id) DO NOTHING;
