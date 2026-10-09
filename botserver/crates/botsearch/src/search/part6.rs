use super::*;

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseConnection => write!(f, "Database connection failed"),
            Self::QueryFailed(msg) => write!(f, "Query failed: {msg}"),
            Self::IndexFailed(msg) => write!(f, "Index failed: {msg}"),
            Self::DeleteFailed(msg) => write!(f, "Delete failed: {msg}"),
            Self::InvalidQuery(msg) => write!(f, "Invalid query: {msg}"),
        }
    }
}
impl std::error::Error for SearchError {}
pub fn create_search_index_migration() -> &'static str {
    r#"
    CREATE TABLE IF NOT EXISTS search_index (
        id TEXT NOT NULL,
        source TEXT NOT NULL,
        organization_id UUID NOT NULL,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        url TEXT,
        metadata JSONB DEFAULT '{}',
        search_vector TSVECTOR,
        created_at TIMESTAMPTZ NOT NULL,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (id, source)
    );

    CREATE INDEX IF NOT EXISTS idx_search_index_org ON search_index(organization_id);
    CREATE INDEX IF NOT EXISTS idx_search_index_source ON search_index(source);
    CREATE INDEX IF NOT EXISTS idx_search_index_vector ON search_index USING GIN(search_vector);
    CREATE INDEX IF NOT EXISTS idx_search_index_created ON search_index(created_at);

    CREATE OR REPLACE FUNCTION search_index_update_trigger() RETURNS trigger AS $$
    BEGIN
        NEW.search_vector :=
            setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
            setweight(to_tsvector('english', COALESCE(NEW.content, '')), 'B');
        NEW.updated_at := NOW();
        RETURN NEW;
    END
    $$ LANGUAGE plpgsql;

    DROP TRIGGER IF EXISTS search_index_vector_update ON search_index;
    CREATE TRIGGER search_index_vector_update
        BEFORE INSERT OR UPDATE ON search_index
        FOR EACH ROW EXECUTE FUNCTION search_index_update_trigger();
    "#
}
