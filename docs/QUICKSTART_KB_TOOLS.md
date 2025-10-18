# Quick Start: KB and Tools System

## 🎯 Overview

O sistema KB (Knowledge Base) e Tools é completamente **automático e dirigido pelo Drive**:

- **Monitora o Drive (MinIO/S3)** automaticamente
- **Compila tools** quando `.bas` é alterado em `.gbdialog/`
- **Indexa documentos** quando arquivos mudam em `.gbkb/`
- **KB por usuário**, não por sessão
- **Tools por sessão**, não compilados no runtime

## 🚀 Quick Setup (5 minutos)

### 1. Install Dependencies

```bash
# Start required services
docker-compose up -d qdrant postgres

# MinIO (or S3-compatible storage)
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"
```

### 2. Configure Environment

```bash
# .env
QDRANT_URL=http://localhost:6333
LLM_URL=http://localhost:8081
DRIVE_ENDPOINT=localhost:9000
DRIVE_ACCESS_KEY=minioadmin
DRIVE_SECRET_KEY=minioadmin
DATABASE_URL=postgresql://user:pass@localhost/botserver
```

**Nota:** Use nomes genéricos como `DRIVE_*` ao invés de `MINIO_*` quando possível.

### 3. Run Database Migration

```sql
-- Run migration (compatível SQLite e Postgres)
sqlite3 botserver.db < migrations/6.0.3.sql
-- ou
psql -d botserver -f migrations/6.0.3.sql
```

### 4. Create Bot Structure in Drive

Create bucket: `org1_default.gbai`

```
org1_default.gbai/
├── .gbkb/                    # Knowledge Base folders
│   ├── enrollpdfs/           # Collection 1 (auto-indexed)
│   │   ├── guide.pdf
│   │   └── requirements.pdf
│   └── productdocs/          # Collection 2 (auto-indexed)
│       └── catalog.pdf
└── .gbdialog/                # BASIC scripts (auto-compiled)
    ├── start.bas
    ├── enrollment.bas
    └── pricing.bas
```

## 📝 Create Your First Tool (2 minutes)

### enrollment.bas

```basic
PARAM name AS string LIKE "John Doe" DESCRIPTION "Full name"
PARAM email AS string LIKE "john@example.com" DESCRIPTION "Email address"

DESCRIPTION "User enrollment process"

SAVE "enrollments.csv", name, email
TALK "Enrolled! You can ask me about enrollment procedures."
RETURN "success"
```

### start.bas

```basic
REM ADD_TOOL apenas ASSOCIA a tool à sessão (não compila!)
REM A compilação acontece automaticamente quando o arquivo muda no Drive
ADD_TOOL "enrollment"
ADD_TOOL "pricing"

REM ADD_KB é por USER, não por sessão
REM Basta existir em .gbkb/ que já está indexado
ADD_KB "enrollpdfs"

TALK "Hi! I can help with enrollment and pricing."
```

## 🔄 How It Works: Drive-First Approach

```
┌─────────────────────────────────────────────────────┐
│  1. Upload file.pdf to .gbkb/enrollpdfs/           │
│     ↓                                                │
│  2. DriveMonitor detecta mudança (30s polling)      │
│     ↓                                                │
│  3. Automaticamente indexa no Qdrant                │
│     ↓                                                │
│  4. Metadados salvos no banco (kb_documents)        │
│     ↓                                                │
│  5. KB está disponível para TODOS os usuários       │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  1. Upload enrollment.bas to .gbdialog/             │
│     ↓                                                │
│  2. DriveMonitor detecta mudança (30s polling)      │
│     ↓                                                │
│  3. Automaticamente compila para .ast               │
│     ↓                                                │
│  4. Gera .mcp.json e .tool.json (se tem PARAM)     │
│     ↓                                                │
│  5. Salvo em ./work/default.gbai/default.gbdialog/  │
│     ↓                                                │
│  6. Metadados salvos no banco (basic_tools)         │
│     ↓                                                │
│  7. Tool compilada e pronta para uso                │
└─────────────────────────────────────────────────────┘
```

## 🎯 Keywords BASIC

### ADD_TOOL (Associa tool à sessão)

```basic
ADD_TOOL "enrollment"  # Apenas o nome, sem .bas
```

**O que faz:**
- Associa a tool **já compilada** com a sessão atual
- NÃO compila (isso é feito automaticamente pelo DriveMonitor)
- Armazena em `session_tool_associations` table

**Importante:** A tool deve existir em `basic_tools` (já compilada).

### ADD_KB (Adiciona KB para o usuário)

```basic
ADD_KB "enrollpdfs"
```

**O que faz:**
- Associa KB com o **usuário** (não sessão!)
- Armazena em `user_kb_associations` table
- KB já deve estar indexado (arquivos em `.gbkb/enrollpdfs/`)

### ADD_WEBSITE (Adiciona website como KB para o usuário)

```basic
ADD_WEBSITE "https://docs.example.com"
```

**O que faz:**
- Faz crawling do website (usa `WebCrawler`)
- Cria KB temporário para o usuário
- Indexa no Qdrant
- Armazena em `user_kb_associations` com `is_website=1`

## 📊 Database Tables (SQLite/Postgres Compatible)

### kb_documents (Metadados de documentos indexados)

```sql
CREATE TABLE kb_documents (
    id TEXT PRIMARY KEY,
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    collection_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    indexed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### basic_tools (Tools compiladas)

```sql
CREATE TABLE basic_tools (
    id TEXT PRIMARY KEY,
    bot_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    ast_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    mcp_json TEXT,
    tool_json TEXT,
    compiled_at TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1
);
```

### user_kb_associations (KB por usuário)

```sql
CREATE TABLE user_kb_associations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    bot_id TEXT NOT NULL,
    kb_name TEXT NOT NULL,
    is_website INTEGER NOT NULL DEFAULT 0,
    website_url TEXT,
    UNIQUE(user_id, bot_id, kb_name)
);
```

### session_tool_associations (Tools por sessão)

```sql
CREATE TABLE session_tool_associations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    added_at TEXT NOT NULL,
    UNIQUE(session_id, tool_name)
);
```

## 🔧 Drive Monitor (Automatic Background Service)

O `DriveMonitor` roda automaticamente ao iniciar o servidor:

```rust
// In main.rs
let bucket_name = format!("{}default.gbai", cfg.org_prefix);
let drive_monitor = Arc::new(DriveMonitor::new(app_state, bucket_name));
let _handle = drive_monitor.spawn();
```

**Monitora:**
- `.gbdialog/*.bas` → Compila automaticamente
- `.gbkb/*/*.{pdf,txt,md}` → Indexa automaticamente

**Intervalo:** 30 segundos (ajustável)

## 📚 Example: Complete Enrollment Flow

### 1. Upload enrollment.bas to Drive

```bash
mc cp enrollment.bas local/org1_default.gbai/.gbdialog/
```

### 2. Wait for Compilation (30s max)

```
[INFO] New BASIC tool detected: .gbdialog/enrollment.bas
[INFO] Tool compiled successfully: enrollment
[INFO]   AST: ./work/default.gbai/default.gbdialog/enrollment.ast
[INFO]   MCP tool definition generated
```

### 3. Upload KB documents

```bash
mc cp guide.pdf local/org1_default.gbai/.gbkb/enrollpdfs/
mc cp faq.pdf local/org1_default.gbai/.gbkb/enrollpdfs/
```

### 4. Wait for Indexing (30s max)

```
[INFO] New KB document detected: .gbkb/enrollpdfs/guide.pdf
[INFO] Extracted 5420 characters from .gbkb/enrollpdfs/guide.pdf
[INFO] Document indexed successfully: .gbkb/enrollpdfs/guide.pdf
```

### 5. Use in BASIC Script

```basic
REM start.bas
ADD_TOOL "enrollment"
ADD_KB "enrollpdfs"

TALK "Ready to help with enrollment!"
```

### 6. User Interaction

```
User: "I want to enroll"
Bot: [Calls enrollment tool, collects info]

User: "What documents do I need?"
Bot: [Searches enrollpdfs KB, returns relevant info from guide.pdf]
```

## 🎓 Best Practices

### ✅ DO

- Upload files to Drive and let the system auto-compile/index
- Use generic names (Drive, Cache) when possible
- Use `ADD_KB` for persistent user knowledge
- Use `ADD_TOOL` to activate tools in session
- Keep tools in `.gbdialog/`, KB docs in `.gbkb/`

### ❌ DON'T

- Don't try to compile tools in runtime (it's automatic!)
- Don't use session for KB (it's user-based)
- Don't use `SET_KB` and `ADD_KB` together (they do the same)
- Don't expect instant updates (30s polling interval)

## 🔍 Monitoring

### Check Compiled Tools

```bash
ls -la ./work/default.gbai/default.gbdialog/
# Should see:
# - enrollment.ast
# - enrollment.mcp.json
# - enrollment.tool.json
# - pricing.ast
# - pricing.mcp.json
# - pricing.tool.json
```

### Check Indexed Documents

```bash
# Query Qdrant
curl http://localhost:6333/collections

# Should see collections like:
# - kb_default_enrollpdfs
# - kb_default_productdocs
```

### Check Database

```sql
-- Compiled tools
SELECT tool_name, compiled_at, is_active FROM basic_tools;

-- Indexed documents
SELECT file_path, indexed_at FROM kb_documents;

-- User KBs
SELECT user_id, kb_name, is_website FROM user_kb_associations;

-- Session tools
SELECT session_id, tool_name FROM session_tool_associations;
```

## 🐛 Troubleshooting

### Tool not compiling?

1. Check file is in `.gbdialog/` folder
2. File must end with `.bas`
3. Wait 30 seconds for DriveMonitor poll
4. Check logs: `grep "Compiling BASIC tool" botserver.log`

### Document not indexing?

1. Check file is in `.gbkb/collection_name/` folder
2. File must be `.pdf`, `.txt`, or `.md`
3. Wait 30 seconds for DriveMonitor poll
4. Check logs: `grep "Indexing KB document" botserver.log`

### ADD_TOOL fails?

1. Tool must be already compiled (check `basic_tools` table)
2. Use only tool name: `ADD_TOOL "enrollment"` (not `.bas`)
3. Check if `is_active=1` in database

### KB search not working?

1. Use `ADD_KB` in user's script (not session)
2. Check collection exists in Qdrant
3. Verify `user_kb_associations` has entry
4. Check answer_mode (use 2 or 4 for KB)

## 🆘 Support

- Full Docs: `docs/KB_AND_TOOLS.md`
- Examples: `examples/`
- Deployment: `docs/DEPLOYMENT_CHECKLIST.md`

---

**The system is fully automatic and drive-first!** 🚀

Just upload to Drive → DriveMonitor handles the rest.