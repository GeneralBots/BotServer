-- Migration 6.0.3: KB and Tools tables (SQLite and Postgres compatible)
-- No triggers, no functions, pure table definitions

-- Table for KB documents metadata
CREATE TABLE IF NOT EXISTS kb_documents (
    id TEXT PRIMARY KEY,
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    collection_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    file_hash TEXT NOT NULL,
    first_published_at TEXT NOT NULL,
    last_modified_at TEXT NOT NULL,
    indexed_at TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(bot_id, user_id, collection_name, file_path)
);

CREATE INDEX IF NOT EXISTS idx_kb_documents_bot_id ON kb_documents(bot_id);
CREATE INDEX IF NOT EXISTS idx_kb_documents_user_id ON kb_documents(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_documents_collection ON kb_documents(collection_name);
CREATE INDEX IF NOT EXISTS idx_kb_documents_hash ON kb_documents(file_hash);
CREATE INDEX IF NOT EXISTS idx_kb_documents_indexed_at ON kb_documents(indexed_at);

-- Table for KB collections (per user)
CREATE TABLE IF NOT EXISTS kb_collections (
    id TEXT PRIMARY KEY,
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    folder_path TEXT NOT NULL,
    qdrant_collection TEXT NOT NULL,
    document_count INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(bot_id, user_id, name)
);

CREATE INDEX IF NOT EXISTS idx_kb_collections_bot_id ON kb_collections(bot_id);
CREATE INDEX IF NOT EXISTS idx_kb_collections_user_id ON kb_collections(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_collections_name ON kb_collections(name);
CREATE INDEX IF NOT EXISTS idx_kb_collections_active ON kb_collections(is_active);

-- Table for compiled BASIC tools
CREATE TABLE IF NOT EXISTS basic_tools (
    id TEXT PRIMARY KEY,
    bot_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    ast_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    mcp_json TEXT,
    tool_json TEXT,
    compiled_at TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(bot_id, tool_name)
);

CREATE INDEX IF NOT EXISTS idx_basic_tools_bot_id ON basic_tools(bot_id);
CREATE INDEX IF NOT EXISTS idx_basic_tools_name ON basic_tools(tool_name);
CREATE INDEX IF NOT EXISTS idx_basic_tools_active ON basic_tools(is_active);
CREATE INDEX IF NOT EXISTS idx_basic_tools_hash ON basic_tools(file_hash);

-- Table for user KB associations (which KBs are active for a user)
CREATE TABLE IF NOT EXISTS user_kb_associations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    bot_id TEXT NOT NULL,
    kb_name TEXT NOT NULL,
    is_website INTEGER NOT NULL DEFAULT 0,
    website_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, bot_id, kb_name)
);

CREATE INDEX IF NOT EXISTS idx_user_kb_user_id ON user_kb_associations(user_id);
CREATE INDEX IF NOT EXISTS idx_user_kb_bot_id ON user_kb_associations(bot_id);
CREATE INDEX IF NOT EXISTS idx_user_kb_name ON user_kb_associations(kb_name);
CREATE INDEX IF NOT EXISTS idx_user_kb_website ON user_kb_associations(is_website);

-- Table for session tool associations (which tools are available in a session)
CREATE TABLE IF NOT EXISTS session_tool_associations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    added_at TEXT NOT NULL,
    UNIQUE(session_id, tool_name)
);

CREATE INDEX IF NOT EXISTS idx_session_tool_session ON session_tool_associations(session_id);
CREATE INDEX IF NOT EXISTS idx_session_tool_name ON session_tool_associations(tool_name);
