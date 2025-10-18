# Knowledge Base (KB) and Tools System

## Overview

This document describes the comprehensive Knowledge Base (KB) and BASIC Tools compilation system integrated into the botserver. This system enables:

1. **Dynamic Knowledge Base Management**: Monitor MinIO buckets for document changes and automatically index them in Qdrant vector database
2. **BASIC Tool Compilation**: Compile BASIC scripts into AST and generate MCP/OpenAI tool definitions
3. **Intelligent Context Processing**: Enhance prompts with relevant KB documents and available tools based on answer mode
4. **Temporary Website Indexing**: Crawl and index web pages for session-specific knowledge

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Bot Server                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ KB Manager   │    │ MinIO        │    │ Qdrant       │      │
│  │              │◄──►│ Handler      │◄──►│ Client       │      │
│  │              │    │              │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                                        ▲               │
│         │                                        │               │
│         ▼                                        │               │
│  ┌──────────────┐                        ┌──────────────┐      │
│  │ BASIC        │                        │ Embeddings   │      │
│  │ Compiler     │                        │ Generator    │      │
│  │              │                        │              │      │
│  └──────────────┘                        └──────────────┘      │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────────────────────────────────────────────┐      │
│  │         Prompt Processor                              │      │
│  │  (Integrates KB + Tools based on Answer Mode)        │      │
│  └──────────────────────────────────────────────────────┘      │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. KB Manager (`src/kb/mod.rs`)

The KB Manager coordinates MinIO monitoring and Qdrant indexing:

- **Watches collections**: Monitors `.gbkb/` folders for document changes
- **Detects changes**: Uses file hashing (SHA256) to detect modified files
- **Indexes documents**: Splits documents into chunks and generates embeddings
- **Stores metadata**: Maintains document information in PostgreSQL

#### Key Functions

```rust
// Add a KB collection to be monitored
kb_manager.add_collection(bot_id, "enrollpdfs").await?;

// Remove a collection
kb_manager.remove_collection("enrollpdfs").await?;

// Start the monitoring service
let kb_handle = kb_manager.spawn();
```

### 2. MinIO Handler (`src/kb/minio_handler.rs`)

Monitors MinIO buckets for file changes:

- **Polling**: Checks for changes every 15 seconds
- **Event detection**: Identifies created, modified, and deleted files
- **State tracking**: Maintains file ETags and sizes for change detection

#### File Change Events

```rust
pub enum FileChangeEvent {
    Created { path: String, size: i64, etag: String },
    Modified { path: String, size: i64, etag: String },
    Deleted { path: String },
}
```

### 3. Qdrant Client (`src/kb/qdrant_client.rs`)

Manages vector database operations:

- **Collection management**: Create, delete, and check collections
- **Point operations**: Upsert and delete vector points
- **Search**: Semantic search using cosine similarity

#### Example Usage

```rust
let client = get_qdrant_client(&state)?;

// Create collection
client.create_collection("kb_bot123_enrollpdfs", 1536).await?;

// Search
let results = client.search("kb_bot123_enrollpdfs", query_vector, 5).await?;
```

### 4. Embeddings Generator (`src/kb/embeddings.rs`)

Handles text embedding and document indexing:

- **Chunking**: Splits documents into 512-character chunks with 50-char overlap
- **Embedding**: Generates vectors using local LLM server
- **Indexing**: Stores chunks with metadata in Qdrant

#### Document Processing

```rust
// Index a document
index_document(&state, "kb_bot_collection", "file.pdf", &content).await?;

// Search for similar documents
let results = search_similar(&state, "kb_bot_collection", "query", 5).await?;
```

### 5. BASIC Compiler (`src/basic/compiler/mod.rs`)

Compiles BASIC scripts and generates tool definitions:

#### Input: BASIC Script with Metadata

```basic
PARAM name AS string LIKE "Abreu Silva" DESCRIPTION "Required full name"
PARAM birthday AS date LIKE "23/09/2001" DESCRIPTION "Birth date in DD/MM/YYYY"
PARAM email AS string LIKE "user@example.com" DESCRIPTION "Email address"

DESCRIPTION "Enrollment process for new users"

// Script logic here
SAVE "enrollments.csv", id, name, birthday, email
TALK "Thanks, you are enrolled!"
SET_KB "enrollpdfs"
```

#### Output: Multiple Files

1. **enrollment.ast**: Compiled Rhai AST
2. **enrollment.mcp.json**: MCP tool definition
3. **enrollment.tool.json**: OpenAI tool definition

#### MCP Tool Format

```json
{
  "name": "enrollment",
  "description": "Enrollment process for new users",
  "input_schema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Required full name",
        "example": "Abreu Silva"
      },
      "birthday": {
        "type": "string",
        "description": "Birth date in DD/MM/YYYY",
        "example": "23/09/2001"
      }
    },
    "required": ["name", "birthday", "email"]
  }
}
```

### 6. Prompt Processor (`src/context/prompt_processor.rs`)

Enhances queries with context based on answer mode:

#### Answer Modes

| Mode | Value | Description |
|------|-------|-------------|
| Direct | 0 | No additional context, direct LLM response |
| WithTools | 1 | Include available tools in prompt |
| DocumentsOnly | 2 | Search KB only, no LLM generation |
| WebSearch | 3 | Include web search results |
| Mixed | 4 | Combine KB documents + tools (context-aware) |

#### Mixed Mode Flow

```
User Query
    │
    ▼
┌─────────────────────────┐
│  Prompt Processor       │
│  (Answer Mode: Mixed)   │
└─────────────────────────┘
    │
    ├──► Search KB Documents (Qdrant)
    │    └─► Returns relevant chunks
    │
    ├──► Get Available Tools (Session Context)
    │    └─► Returns tool definitions
    │
    ▼
┌─────────────────────────┐
│  Enhanced Prompt        │
│  • System Prompt        │
│  • Document Context     │
│  • Available Tools      │
│  • User Query           │
└─────────────────────────┘
```

## BASIC Keywords

### SET_KB

Activates a KB collection for the current session.

```basic
SET_KB "enrollpdfs"
```

- Creates/ensures Qdrant collection exists
- Updates session context with active collection
- Documents in `.gbkb/enrollpdfs/` are indexed

### ADD_KB

Adds an additional KB collection (can have multiple).

```basic
ADD_KB "productbrochurespdfsanddocs"
```

### ADD_TOOL

Compiles and registers a BASIC tool.

```basic
ADD_TOOL "enrollment.bas"
```

Downloads from MinIO (`.gbdialog/enrollment.bas`), compiles to:
- `./work/{bot_id}.gbai/{bot_id}.gbdialog/enrollment.ast`
- `./work/{bot_id}.gbai/{bot_id}.gbdialog/enrollment.mcp.json`
- `./work/{bot_id}.gbai/{bot_id}.gbdialog/enrollment.tool.json`

#### With MCP Endpoint

```basic
ADD_TOOL "enrollment.bas" as MCP
```

Creates an HTTP endpoint at `/default/enrollment` that:
- Accepts JSON matching the tool schema
- Executes the BASIC script
- Returns the result

### ADD_WEBSITE

Crawls and indexes a website for the current session.

```basic
ADD_WEBSITE "https://example.com/docs"
```

- Fetches HTML content
- Extracts readable text (removes scripts, styles)
- Creates temporary Qdrant collection
- Indexes content with embeddings
- Available for remainder of session

## Database Schema

### kb_documents

Stores metadata about indexed documents:

```sql
CREATE TABLE kb_documents (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    collection_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    file_hash TEXT NOT NULL,
    first_published_at TIMESTAMPTZ NOT NULL,
    last_modified_at TIMESTAMPTZ NOT NULL,
    indexed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    UNIQUE(bot_id, collection_name, file_path)
);
```

### kb_collections

Stores KB collection information:

```sql
CREATE TABLE kb_collections (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    name TEXT NOT NULL,
    folder_path TEXT NOT NULL,
    qdrant_collection TEXT NOT NULL,
    document_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(bot_id, name)
);
```

### basic_tools

Stores compiled BASIC tools:

```sql
CREATE TABLE basic_tools (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    ast_path TEXT NOT NULL,
    mcp_json JSONB,
    tool_json JSONB,
    compiled_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE(bot_id, tool_name)
);
```

## Workflow Examples

### Example 1: Enrollment with KB

**File Structure:**
```
bot.gbai/
├── .gbkb/
│   └── enrollpdfs/
│       ├── enrollment_guide.pdf
│       ├── requirements.pdf
│       └── faq.pdf
├── .gbdialog/
│   ├── start.bas
│   └── enrollment.bas
```

**start.bas:**
```basic
ADD_TOOL "enrollment.bas" as MCP
ADD_KB "enrollpdfs"
```

**enrollment.bas:**
```basic
PARAM name AS string LIKE "John Doe" DESCRIPTION "Full name"
PARAM email AS string LIKE "john@example.com" DESCRIPTION "Email"

DESCRIPTION "Enrollment process with KB support"

// Validate input
IF name = "" THEN
    TALK "Please provide your name"
    EXIT
END IF

// Save to database
SAVE "enrollments.csv", name, email

// Set KB for enrollment docs
SET_KB "enrollpdfs"

TALK "Thanks! You can now ask me about enrollment procedures."
```

**User Interaction:**
1. User: "I want to enroll"
2. Bot calls `enrollment` tool, collects parameters
3. After enrollment, SET_KB activates `enrollpdfs` collection
4. User: "What documents do I need?"
5. Bot searches KB (mode=2 or 4), finds relevant PDFs, responds with info

### Example 2: Product Support with Web Content

**support.bas:**
```basic
PARAM product AS string LIKE "fax" DESCRIPTION "Product name"

DESCRIPTION "Get product information"

// Find in database
price = -1
productRecord = FIND "products.csv", "name = ${product}"
IF productRecord THEN
    price = productRecord.price
END IF

// Add product documentation website
ADD_WEBSITE "https://example.com/products/${product}"

// Add product brochures KB
SET_KB "productbrochurespdfsanddocs"

RETURN price
```

**User Flow:**
1. User: "What's the price of a fax machine?"
2. Tool executes, finds price in CSV
3. ADD_WEBSITE indexes product page
4. SET_KB activates brochures collection
5. User: "How do I set it up?"
6. Prompt processor (Mixed mode) searches both:
   - Temporary website collection
   - Product brochures KB
7. Returns setup instructions from indexed sources

## Configuration

### Environment Variables

```bash
# Qdrant Configuration
QDRANT_URL=http://localhost:6333

# LLM for Embeddings
LLM_URL=http://localhost:8081

# MinIO Configuration (from config)
MINIO_ENDPOINT=localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin
MINIO_ORG_PREFIX=org1_

# Database
DATABASE_URL=postgresql://user:pass@localhost/botserver
```

### Answer Mode Selection

Set in session's `answer_mode` field:

```rust
// Example: Update session to Mixed mode
session.answer_mode = 4;
```

Or via API when creating session:

```json
POST /sessions
{
  "user_id": "...",
  "bot_id": "...",
  "answer_mode": 4
}
```

## Security Considerations

1. **Path Traversal Protection**: All file paths validated to prevent `..` attacks
2. **Safe Tool Paths**: Tools must be in `.gbdialog/` folder
3. **URL Validation**: ADD_WEBSITE only allows HTTP/HTTPS URLs
4. **Bucket Isolation**: Each organization has separate MinIO bucket
5. **Hash Verification**: File changes detected by SHA256 hash

## Performance Tuning

### KB Manager

- **Poll Interval**: 30 seconds (adjustable in `kb/mod.rs`)
- **Chunk Size**: 512 characters (in `kb/embeddings.rs`)
- **Chunk Overlap**: 50 characters

### MinIO Handler

- **Poll Interval**: 15 seconds (adjustable in `kb/minio_handler.rs`)
- **State Caching**: File states cached in memory

### Qdrant

- **Vector Size**: 1536 (OpenAI ada-002 compatible)
- **Distance Metric**: Cosine similarity
- **Search Limit**: Configurable per query

## Troubleshooting

### Documents Not Indexing

1. Check MinIO handler is watching correct prefix:
   ```rust
   minio_handler.watch_prefix(".gbkb/").await;
   ```

2. Verify Qdrant connection:
   ```bash
   curl http://localhost:6333/collections
   ```

3. Check logs for indexing errors:
   ```
   grep "Indexing document" botserver.log
   ```

### Tools Not Compiling

1. Verify PARAM syntax is correct
2. Check tool file is in `.gbdialog/` folder
3. Ensure work directory exists and is writable
4. Review compilation logs

### KB Search Not Working

1. Verify collection exists in session context
2. Check Qdrant collection created:
   ```bash
   curl http://localhost:6333/collections/{collection_name}
   ```
3. Ensure embeddings are being generated (check LLM server)

## Future Enhancements

1. **Incremental Indexing**: Only reindex changed chunks
2. **Document Deduplication**: Detect and merge duplicate content
3. **Advanced Crawling**: Follow links, handle JavaScript
4. **Tool Versioning**: Track tool versions and changes
5. **KB Analytics**: Track search queries and document usage
6. **Automatic Tool Discovery**: Scan `.gbdialog/` on startup
7. **Distributed Indexing**: Scale across multiple workers
8. **Real-time Notifications**: WebSocket updates when KB changes

## References

- **Qdrant Documentation**: https://qdrant.tech/documentation/
- **Model Context Protocol**: https://modelcontextprotocol.io/
- **MinIO Documentation**: https://min.io/docs/
- **Rhai Scripting**: https://rhai.rs/book/

## Support

For issues or questions:
- GitHub Issues: https://github.com/GeneralBots/BotServer/issues
- Documentation: https://docs.generalbots.ai/