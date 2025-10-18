# KB and Tools System - Deployment Checklist

## 🎯 Pre-Deployment Checklist

### Infrastructure Requirements

- [ ] **PostgreSQL 12+** running and accessible
- [ ] **Qdrant** vector database running (port 6333)
- [ ] **MinIO** object storage running (ports 9000, 9001)
- [ ] **LLM Server** for embeddings (port 8081)
- [ ] **Redis** (optional, for caching)

### System Resources

- [ ] **Minimum 4GB RAM** (8GB recommended)
- [ ] **10GB disk space** for documents and embeddings
- [ ] **2+ CPU cores** for parallel processing
- [ ] **Network access** to external APIs (if using ADD_WEBSITE)

---

## 📋 Configuration Steps

### 1. Environment Variables

Create/update `.env` file:

```bash
# Core Settings
DATABASE_URL=postgresql://user:pass@localhost:5432/botserver
QDRANT_URL=http://localhost:6333
LLM_URL=http://localhost:8081
CACHE_URL=redis://127.0.0.1/

# MinIO Configuration
MINIO_ENDPOINT=localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin
MINIO_USE_SSL=false
MINIO_ORG_PREFIX=org1_

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
RUST_LOG=info
```

**Verify:**
- [ ] All URLs are correct and accessible
- [ ] Credentials are set properly
- [ ] Org prefix matches your organization

---

### 2. Database Setup

```bash
# Connect to PostgreSQL
psql -U postgres -d botserver

# Run migration
\i migrations/create_kb_and_tools_tables.sql

# Verify tables created
\dt kb_*
\dt basic_tools

# Check triggers
\df update_updated_at_column
```

**Verify:**
- [ ] Tables `kb_documents`, `kb_collections`, `basic_tools` exist
- [ ] Indexes are created
- [ ] Triggers are active
- [ ] No migration errors

---

### 3. MinIO Bucket Setup

```bash
# Using MinIO CLI (mc)
mc alias set local http://localhost:9000 minioadmin minioadmin
mc mb local/org1_default.gbai
mc policy set public local/org1_default.gbai

# Or via MinIO Console at http://localhost:9001
```

**Create folder structure:**
```
org1_default.gbai/
├── .gbkb/          # Knowledge Base documents
└── .gbdialog/      # BASIC scripts
```

**Verify:**
- [ ] Bucket created with correct name
- [ ] Folders `.gbkb/` and `.gbdialog/` exist
- [ ] Upload permissions work
- [ ] Download/read permissions work

---

### 4. Qdrant Setup

```bash
# Check Qdrant is running
curl http://localhost:6333/

# Expected response: {"title":"qdrant - vector search engine","version":"..."}
```

**Verify:**
- [ ] Qdrant responds on port 6333
- [ ] API is accessible
- [ ] Dashboard works at http://localhost:6333/dashboard
- [ ] No authentication errors

---

### 5. LLM Server for Embeddings

```bash
# Check LLM server is running
curl http://localhost:8081/v1/models

# Test embeddings endpoint
curl -X POST http://localhost:8081/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input": ["test"], "model": "text-embedding-ada-002"}'
```

**Verify:**
- [ ] LLM server responds
- [ ] Embeddings endpoint works
- [ ] Vector dimension is 1536 (or update in code)
- [ ] Response time < 5 seconds

---

## 🚀 Deployment

### 1. Build Application

```bash
# Clean build
cargo clean
cargo build --release

# Verify binary
./target/release/botserver --version
```

**Verify:**
- [ ] Compilation succeeds with no errors
- [ ] Binary created in `target/release/`
- [ ] All features enabled correctly

---

### 2. Upload Initial Files

**Upload to MinIO `.gbkb/` folder:**
```bash
# Example: Upload enrollment documents
mc cp enrollment_guide.pdf local/org1_default.gbai/.gbkb/enrollpdfs/
mc cp requirements.pdf local/org1_default.gbai/.gbkb/enrollpdfs/
mc cp faq.pdf local/org1_default.gbai/.gbkb/enrollpdfs/
```

**Upload to MinIO `.gbdialog/` folder:**
```bash
# Upload BASIC tools
mc cp start.bas local/org1_default.gbai/.gbdialog/
mc cp enrollment.bas local/org1_default.gbai/.gbdialog/
mc cp pricing.bas local/org1_default.gbai/.gbdialog/
```

**Verify:**
- [ ] Documents uploaded successfully
- [ ] BASIC scripts uploaded
- [ ] Files are readable via MinIO
- [ ] Correct folder structure maintained

---

### 3. Start Services

```bash
# Start botserver
./target/release/botserver

# Or with systemd
sudo systemctl start botserver
sudo systemctl enable botserver

# Or with Docker
docker-compose up -d botserver
```

**Monitor startup logs:**
```bash
# Check logs
tail -f /var/log/botserver.log

# Or Docker logs
docker logs -f botserver
```

**Look for:**
- [ ] `KB Manager service started`
- [ ] `MinIO Handler service started`
- [ ] `Startup complete!`
- [ ] No errors about missing services

---

### 4. Verify KB Indexing

**Wait 30-60 seconds for initial indexing**

```bash
# Check Qdrant collections
curl http://localhost:6333/collections

# Should see collections like:
# - kb_<bot_id>_enrollpdfs
# - kb_<bot_id>_productdocs
```

**Check logs for indexing:**
```bash
grep "Indexing document" /var/log/botserver.log
grep "Document indexed successfully" /var/log/botserver.log
```

**Verify:**
- [ ] Collections created in Qdrant
- [ ] Documents indexed (check chunk count)
- [ ] No indexing errors in logs
- [ ] File hashes stored in database

---

### 5. Test Tool Compilation

**Check compiled tools:**
```bash
# List work directory
ls -la ./work/*/default.gbdialog/

# Should see:
# - *.ast files (compiled AST)
# - *.mcp.json files (MCP definitions)
# - *.tool.json files (OpenAI definitions)
```

**Verify:**
- [ ] AST files created for each .bas file
- [ ] MCP JSON files generated (if PARAM exists)
- [ ] Tool JSON files generated (if PARAM exists)
- [ ] No compilation errors in logs

---

## 🧪 Testing

### Test 1: KB Search

```bash
# Create test session with answer_mode=2 (documents only)
curl -X POST http://localhost:8080/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test-user",
    "bot_id": "default",
    "answer_mode": 2
  }'

# Send query
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "<session_id>",
    "message": "What documents do I need for enrollment?"
  }'
```

**Expected:**
- [ ] Response contains information from indexed PDFs
- [ ] References to source documents
- [ ] Relevant chunks retrieved

---

### Test 2: Tool Calling

```bash
# Call enrollment tool endpoint
curl -X POST http://localhost:8080/default/enrollment \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test User",
    "email": "test@example.com"
  }'
```

**Expected:**
- [ ] Tool executes successfully
- [ ] Data saved to CSV
- [ ] Response includes enrollment ID
- [ ] KB activated (if SET_KB in script)

---

### Test 3: Mixed Mode (KB + Tools)

```bash
# Create session with answer_mode=4 (mixed)
curl -X POST http://localhost:8080/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test-user",
    "bot_id": "default",
    "answer_mode": 4
  }'

# Send query that should use both KB and tools
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "<session_id>",
    "message": "I want to enroll. What information do you need?"
  }'
```

**Expected:**
- [ ] Bot references both KB documents and available tools
- [ ] Intelligently decides when to use KB vs tools
- [ ] Context includes both document excerpts and tool info

---

### Test 4: Website Indexing

```bash
# In BASIC or via API, test ADD_WEBSITE
# (Requires script with ADD_WEBSITE keyword)

# Check temporary collection created
curl http://localhost:6333/collections | grep temp_website
```

**Expected:**
- [ ] Website crawled successfully
- [ ] Temporary collection created
- [ ] Content indexed
- [ ] Available for current session only

---

## 🔍 Monitoring

### Health Checks

```bash
# Botserver health
curl http://localhost:8080/health

# Qdrant health
curl http://localhost:6333/

# MinIO health
curl http://localhost:9000/minio/health/live

# Database connection
psql -U postgres -d botserver -c "SELECT 1"
```

**Set up alerts for:**
- [ ] Service downtime
- [ ] High memory usage (>80%)
- [ ] Disk space low (<10%)
- [ ] Indexing failures
- [ ] Tool compilation errors

---

### Log Monitoring

**Important log patterns to watch:**

```bash
# Successful indexing
grep "Document indexed successfully" botserver.log

# Indexing errors
grep "ERROR.*Indexing" botserver.log

# Tool compilation
grep "Tool compiled successfully" botserver.log

# KB Manager activity
grep "KB Manager" botserver.log

# MinIO handler activity
grep "MinIO Handler" botserver.log
```

---

### Database Monitoring

```sql
-- Check document count per collection
SELECT collection_name, COUNT(*) as doc_count 
FROM kb_documents 
GROUP BY collection_name;

-- Check indexing status
SELECT 
  collection_name,
  COUNT(*) as total,
  COUNT(indexed_at) as indexed,
  COUNT(*) - COUNT(indexed_at) as pending
FROM kb_documents 
GROUP BY collection_name;

-- Check compiled tools
SELECT tool_name, compiled_at, is_active 
FROM basic_tools 
ORDER BY compiled_at DESC;

-- Recent KB activity
SELECT * FROM kb_documents 
ORDER BY updated_at DESC 
LIMIT 10;
```

---

## 🔒 Security Checklist

- [ ] Change default MinIO credentials
- [ ] Enable SSL/TLS for MinIO
- [ ] Set up firewall rules
- [ ] Enable Qdrant authentication
- [ ] Use secure PostgreSQL connections
- [ ] Validate file uploads (size, type)
- [ ] Implement rate limiting
- [ ] Set up proper CORS policies
- [ ] Use environment variables for secrets
- [ ] Enable request logging
- [ ] Set up backup strategy

---

## 📊 Performance Tuning

### MinIO Handler
```rust
// In src/kb/minio_handler.rs
interval(Duration::from_secs(15))  // Adjust polling interval
```

### KB Manager
```rust
// In src/kb/mod.rs
interval(Duration::from_secs(30))  // Adjust check interval
```

### Embeddings
```rust
// In src/kb/embeddings.rs
const CHUNK_SIZE: usize = 512;     // Adjust chunk size
const CHUNK_OVERLAP: usize = 50;   // Adjust overlap
```

### Qdrant
```rust
// In src/kb/qdrant_client.rs
let vector_size = 1536;            // Match your embedding model
```

**Tune based on:**
- [ ] Document update frequency
- [ ] System resource usage
- [ ] Query performance requirements
- [ ] Embedding model characteristics

---

## 🔄 Backup & Recovery

### Database Backup
```bash
# Daily backup
pg_dump -U postgres botserver > botserver_$(date +%Y%m%d).sql

# Restore
psql -U postgres botserver < botserver_20240101.sql
```

### MinIO Backup
```bash
# Backup bucket
mc mirror local/org1_default.gbai/ ./backups/minio/

# Restore
mc mirror ./backups/minio/ local/org1_default.gbai/
```

### Qdrant Backup
```bash
# Snapshot all collections
curl -X POST http://localhost:6333/collections/{collection_name}/snapshots

# Download snapshot
curl http://localhost:6333/collections/{collection_name}/snapshots/{snapshot_name}
```

**Schedule:**
- [ ] Database: Daily at 2 AM
- [ ] MinIO: Daily at 3 AM
- [ ] Qdrant: Weekly
- [ ] Test restore monthly

---

## 📚 Documentation

- [ ] Update API documentation
- [ ] Document custom BASIC keywords
- [ ] Create user guides for tools
- [ ] Document KB collection structure
- [ ] Create troubleshooting guide
- [ ] Document deployment process
- [ ] Create runbooks for common issues

---

## ✅ Post-Deployment Verification

**Final Checklist:**

- [ ] All services running and healthy
- [ ] Documents indexing automatically
- [ ] Tools compiling on upload
- [ ] KB search working correctly
- [ ] Tool endpoints responding
- [ ] Mixed mode working as expected
- [ ] Logs are being written
- [ ] Monitoring is active
- [ ] Backups scheduled
- [ ] Security measures in place
- [ ] Documentation updated
- [ ] Team trained on system

---

## 🆘 Rollback Plan

**If deployment fails:**

1. **Stop services**
   ```bash
   sudo systemctl stop botserver
   ```

2. **Restore database**
   ```bash
   psql -U postgres botserver < botserver_backup.sql
   ```

3. **Restore MinIO**
   ```bash
   mc mirror ./backups/minio/ local/org1_default.gbai/
   ```

4. **Revert code**
   ```bash
   git checkout <previous-version>
   cargo build --release
   ```

5. **Restart services**
   ```bash
   sudo systemctl start botserver
   ```

6. **Verify rollback**
   - Test basic functionality
   - Check logs for errors
   - Verify data integrity

---

## 📞 Support Contacts

- **Infrastructure Issues:** DevOps Team
- **Database Issues:** DBA Team
- **Application Issues:** Development Team
- **Security Issues:** Security Team

---

## 📅 Maintenance Schedule

- **Daily:** Check logs, monitor services
- **Weekly:** Review KB indexing stats, check disk space
- **Monthly:** Test backups, review performance metrics
- **Quarterly:** Security audit, update dependencies

---

**Deployment Status:** ⬜ Not Started | 🟡 In Progress | ✅ Complete

**Deployed By:** ________________  
**Date:** ________________  
**Version:** ________________  
**Sign-off:** ________________