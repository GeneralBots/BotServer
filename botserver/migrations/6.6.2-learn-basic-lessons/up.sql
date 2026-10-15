-- Storage for the learning BASIC keywords (TEACH, STUDY, QUIZ, CURRICULUM,
-- LESSON, PROGRESS). Migration 6.0.5 created `learn_lessons` with the course
-- shape (`course_id`, `title`, ...), so the keyword storage gets its own table
-- instead of colliding on the shared name (#1506).

CREATE TABLE IF NOT EXISTS learn_basic_lessons (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    user_id UUID,
    topic TEXT NOT NULL,
    kind TEXT NOT NULL,
    content JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_learn_basic_lessons_bot_kind
    ON learn_basic_lessons (bot_id, kind);

CREATE INDEX IF NOT EXISTS idx_learn_basic_lessons_user_created
    ON learn_basic_lessons (user_id, created_at DESC);

-- Recover any rows written while the keyword storage still targeted a
-- keyword-shaped `learn_lessons`, when such a table exists.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'learn_lessons' AND column_name = 'bot_id'
    ) THEN
        INSERT INTO learn_basic_lessons (id, bot_id, user_id, topic, kind, content, created_at)
        SELECT id, bot_id, user_id, topic, kind, content, created_at FROM learn_lessons
        ON CONFLICT (id) DO NOTHING;
    END IF;
END $$;
