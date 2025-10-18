-- Migration: Create KB and Tools tables
-- Description: Tables for Knowledge Base management and BASIC tools compilation

-- Table for KB documents metadata
CREATE TABLE IF NOT EXISTS kb_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL,
    collection_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,
    file_hash TEXT NOT NULL,
    first_published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    indexed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, collection_name, file_path)
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_kb_documents_bot_id ON kb_documents(bot_id);
CREATE INDEX IF NOT EXISTS idx_kb_documents_collection ON kb_documents(collection_name);
CREATE INDEX IF NOT EXISTS idx_kb_documents_hash ON kb_documents(file_hash);
CREATE INDEX IF NOT EXISTS idx_kb_documents_indexed_at ON kb_documents(indexed_at);

-- Table for KB collections
CREATE TABLE IF NOT EXISTS kb_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL,
    name TEXT NOT NULL,
    folder_path TEXT NOT NULL,
    qdrant_collection TEXT NOT NULL,
    document_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, name)
);

-- Index for KB collections
CREATE INDEX IF NOT EXISTS idx_kb_collections_bot_id ON kb_collections(bot_id);
CREATE INDEX IF NOT EXISTS idx_kb_collections_name ON kb_collections(name);

-- Table for compiled BASIC tools
CREATE TABLE IF NOT EXISTS basic_tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id UUID NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    ast_path TEXT NOT NULL,
    mcp_json JSONB,
    tool_json JSONB,
    compiled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, tool_name)
);

-- Index for BASIC tools
CREATE INDEX IF NOT EXISTS idx_basic_tools_bot_id ON basic_tools(bot_id);
CREATE INDEX IF NOT EXISTS idx_basic_tools_name ON basic_tools(tool_name);
CREATE INDEX IF NOT EXISTS idx_basic_tools_active ON basic_tools(is_active);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updating updated_at
DROP TRIGGER IF EXISTS update_kb_documents_updated_at ON kb_documents;
CREATE TRIGGER update_kb_documents_updated_at
    BEFORE UPDATE ON kb_documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_kb_collections_updated_at ON kb_collections;
CREATE TRIGGER update_kb_collections_updated_at
    BEFORE UPDATE ON kb_collections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_basic_tools_updated_at ON basic_tools;
CREATE TRIGGER update_basic_tools_updated_at
    BEFORE UPDATE ON basic_tools
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE kb_documents IS 'Stores metadata about documents in Knowledge Base collections';
COMMENT ON TABLE kb_collections IS 'Stores information about KB collections and their Qdrant mappings';
COMMENT ON TABLE basic_tools IS 'Stores compiled BASIC tools with their MCP and OpenAI tool definitions';

COMMENT ON COLUMN kb_documents.file_hash IS 'SHA256 hash of file content for change detection';
COMMENT ON COLUMN kb_documents.indexed_at IS 'Timestamp when document was last indexed in Qdrant';
COMMENT ON COLUMN kb_collections.qdrant_collection IS 'Name of corresponding Qdrant collection';
COMMENT ON COLUMN basic_tools.mcp_json IS 'Model Context Protocol tool definition';
COMMENT ON COLUMN basic_tools.tool_json IS 'OpenAI-compatible tool definition';
