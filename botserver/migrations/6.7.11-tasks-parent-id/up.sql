-- `tasks.parent_id` backs the subtask hierarchy (`bottasks` groups rows with a
-- parent under their parent — issue #878), but no migration ever created the
-- column: the UI's `SELECT ... parent_id ...` and the `Task`/`NewTask` structs
-- referenced it, so a fresh database failed those queries. Added last so the
-- physical order of the existing columns is unchanged.

ALTER TABLE tasks
    ADD COLUMN IF NOT EXISTS parent_id UUID REFERENCES tasks(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks(parent_id);
