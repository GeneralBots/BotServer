
CREATE TABLE bot_memories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, key)
);

CREATE INDEX idx_bot_memories_bot_id ON bot_memories(bot_id);
CREATE INDEX idx_bot_memories_key ON bot_memories(key);
