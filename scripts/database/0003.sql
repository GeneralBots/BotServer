CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    bot_id TEXT NOT NULL,
    answer_mode TEXT NOT NULL DEFAULT 'direct',
    context JSONB NOT NULL DEFAULT '{}',
    current_tool TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, bot_id)
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user_bot ON user_sessions(user_id, bot_id);
