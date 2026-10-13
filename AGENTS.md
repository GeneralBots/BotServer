# General Bots AI Agent Guidelines

<p align="center"><img src="logo.svg" alt="General Bots" width="200"></p>

## Non-Negotiable Rules

| Rule | Directive |
|------|-----------|
| **Language** | 🌐 ALWAYS respond in English regardless of the user's language — answer directly and concisely |
| **Temp files** | Stop saving `.png`/files on repo root — use `/tmp/`. NEVER create new files on root |
| **Backups** | NEVER create `.bak`/`.old`/backup dirs in the repo — use `/tmp/` only; never commit them |
| **ALM push** | NEVER push to alm without asking first — it is production! |
| **Deploy** | ❌ NEVER deploy to production manually (no `scp`, no direct SSH binary copy, no manual transfers). ✅ ALWAYS push to ALM → CI builds on alm-ci → CI deploys to system container automatically. Never compile directly for production — ALWAYS push + CI/CD |
| **Secrets** | ❌ NEVER include sensitive data (IPs, tokens, passwords, keys) in AGENTS.md or any documentation. Never hardcode credentials in source — use `generate_random_string()` or env vars. Secret files go in `/tmp/` only |
| **Bot restarts** | ❌ NEVER restart botserver for `config.csv` changes — DriveMonitor auto-reloads on ETag change (~10s) |
| **Branching** | ❌ NEVER change git branches without explicit user approval |
| **Todos** | 📋 Update the todo list REGULARLY — after every completed step, when starting a new phase, and whenever priorities change. Long stretches without todo updates are forbidden |
| **CDP control** | 🖥️ Browser automation MUST be fragmented: one short script = ONE action, print state, STOP. NEVER generate long all-in-one CDP scripts that batch many steps silently |
| **Env status** | I AM IN DEV ENV, but sometimes pasting from PROD — do not treat my env as prod! Just fix, push to CI, so I can test in PROD for a while |
| **License** | The project is **MIT** (relicensed from AGPL-3.0 in `4ef5f58fc`, 2026-09-12) and the copyright holder is **General Bots**. ❌ NEVER describe our own code as AGPL/GPL. ✅ Third-party AGPL notices (MinIO, Stalwart, Forgejo, Garage, Skytable) are correct and MUST stay |

### Ports (quick reference)
8080 = botserver API · 3000 = suite · 4000 = cloud · 5000 = login
- Test suite: http://localhost:3000 | cloud: http://localhost:4000 | login: http://localhost:5000
- Test login here: http://localhost:5000/login
- **Login/Signup exclusivity:** `login.pragmatismo.com.br` (port 5000) is the **only** domain serving login/signup. Port 4000 (cloud) does NOT serve them — `/login` or `/signup` on 4000 redirects to 5000.

### Mandatory Browser Testing
- 🚨 **ALL bot tests MUST be done via browser (Chrome CDP port 9222).** ❌ FORBIDDEN to use WebSocket (node wscat, direct WS scripts) — only the browser reflects the real state of chat, suggestions, buttons, network errors.
- 🚨 EVERY web-facing task (login, dashboard, settings, etc.) MUST be browser-tested before marking complete. **One tab per use case, NEVER close the browser** — tabs are living trace evidence.
- Tool trouble? Go to the official website for proper install/instructions.

### Secrets & IPs
- ❌ NEVER create files with secrets in the repository root. Secret files go in `/tmp/` only (cleared on reboot, not git-tracked, standard Unix practice, prevents accidental commits):
  - ✅ `/tmp/vault-token-gb` — Vault root token
  - ✅ `/tmp/vault-unseal-key-gb` — Vault unseal key
  - ❌ `vault-unseal-keys` — FORBIDDEN (tracked by git)
  - ❌ `start-and-unseal.sh` — FORBIDDEN (contains secrets)
- ❌ NEVER write internal IPs to logs or output. Mask IPs when debugging (e.g. "10.x.x.x" not "10.0.0.1"). Use hostnames instead of IPs in configs and documentation.

### Bot Source Rules
- ✅ `.bas` tool scripts ARE committed in git for git-owned bots (reform #1501/#1505) — the bot's ALM repository is the source of truth, not Drive; `.ast` caches and `work/` copies are build artifacts, never sources
- ✅ `bottemplates/` ships source templates in the monorepo (reference material, not a bot's runtime source)
- ✅ **Git-owned bots (reform #1501/#1505): the ALM repository is the source of truth** — `.gbdialog/` (tools) and `.gbot/` (channel prompts) live in the bot's repo, pulled every 15 s by `git_bot_monitor` and materialized into `work/{branch}.gborg/{branch}.gbai/{bot}.gbdialog|{bot}.gbot/`. Drive copies of those two prefixes are archived to `{bucket}/archive/{bot}-{stamp}/`, never read. `.gbkb`/`.gbdrive` stay in Drive.
- 🚨 **A bot without `PROMPT-{CHANNEL}.md` in its work-dir `.gbot/` silently answers with the generic fallback prompt** (`load_system_prompt_for_channel` in `botserver/src/core/bot/ws/message.rs:16`) — no error is logged, tools simply stop being called (e.g. `classify_media` never fires). Shipped templates must therefore commit their `.gbot` prompts with the tool (`templates.rs` → `config_files`, `persist_config_to_git`, `bot_config::write_config_files`); `git_config` recovers orphaned Drive/archived `.gbot` on boot, and the archive pass refuses to delete a Drive `.gbot` that is not in the repo yet.
- The **runtime** materializes and loads bots through Drive (MinIO `.gbai` buckets) — see `botserver/src/main_module/drive_monitors.rs`. For git-owned bots that Drive tree is a materialized copy of the bot's ALM repo (`git_bot_monitor`, every 15 s); a change made straight in Drive is out-of-band and gets archived. Never point code at local filesystem paths.

### Test Identifiers (prod beiner media-filing E2E)
Reusable ids so the next session does not rediscover them — no secrets, no IPs:

| What | Value |
|------|-------|
| Bot name / display name | `beiner` / "AABB-bot" (Telegram chat handle `aabb_test_bot`) |
| Bot UUID | `f25671b3-61cd-4970-9626-bd99d9fb398a` |
| Web Telegram chat | `https://web.telegram.org/k/#@aabb_test_bot` (tab kept open in the Chrome CDP profile, never closed) |
| Telegram chat id | `6676512312` (user "Rodrigo") |
| Telegram session id | `5f1d25c6-43a0-41c3-bb4a-c61c8c6be42` |
| Bot repo | ALM org `a2125c56`, repo `beiner` — checkout at `/opt/gbo/data/vibe-workspaces/beiner` in the `bot` container |
| Drive buckets | `beiner.gbai` (layout 1, `beiner.gbdrive/…`) and `beiner.gborg` (org layout, `beiner.gbai/…`) |
| E2E result shape | media lands at `beiner.gbdrive/media/{YYYY}/{MM}/{category}/{uuid}.{ext}` + sibling `.meta.txt` (`category=`, `kind=`, `path=`, `caption=`, `perception=`) |
| Prod services | botserver runs via `systemctl` in the `bot` container (logs: `journalctl -u botserver`, health on the container's own port) |

**Send a photo through Telegram Web (fastest reliable path):** Telegram K keeps one hidden `input[type=file]`; setting it directly skips the attach menu. The staged photo opens a `.popup-send-photo` whose own `btn-primary` button must be clicked (`.btn-send` in the composer is covered by the popup):
```python
await page.locator("input[type=file]").first.set_input_files(IMG)   # then wait for .popup-send-photo
await page.locator(".popup-send-photo .btn-primary").first.click()   # send
```


---

## Workspace Structure

### Ports & Services

| Port | Service | Domain | Content | Auth | Routing |
|------|---------|--------|---------|------|---------|
| **3000** | Suite (botui) | `localhost:3000` | `botui/ui/suite/*.html` — HTMX apps, chat, desktop | ✅ GB_LOGIN_URL injected | Reverse proxy → botserver `/api/*`, `/ws` |
| **4000** | Cloud (botui) | `localhost:4000` | `botui/ui/cloud/*.html` — store, dashboard, plans, offers | ❌ No login/signup — redirects → 5000 | URL rewriting (`/store` → `store.html`), GB_LOGIN_URL injected |
| **5000** | Login (botui) | `login.pragmatismo.com.br` | `botui/ui/login/*.html` — login, signup | ✅ Only domain with auth | Serves CSS/JS/images from cloud via proxy |
| **8080** | API (botserver) | `localhost:8080` | API endpoints + fragments | ✅ Bearer token | `/api/*`, `/cloud/partials/*`, `/ws` |
| **—** | Desktop (botapp) | Tauri 2 | Shell wrapper | N/A | N/A |

### Cloud UI — Who Serves What

| Port | Serves | Does Not Serve |
|------|--------|----------------|
| **3000** (suite) | `botui/ui/suite/*` — chat, apps, desktop | ❌ `/cloud/*` (explicit 404) |
| **4000** (cloud) | `botui/ui/cloud/*.html` — store, dashboard, plans, offers | ❌ `/login`, `/signup` (redirects 307 → 5000) |
| **5000** (login) | `botui/ui/login/*.html` — login, signup | Auth only |
| **8080** (botserver) | API (`/api/cloud/*`), fragments (`/cloud/partials/*`), WebSocket | ❌ Complete HTML pages |

**Rule:** botserver NEVER serves complete HTMX pages — only API endpoints and HTML fragments. Complete cloud pages are statically served by botui from `botui/ui/cloud/`.

**`GB_LOGIN_URL` Injection:** Both 3000 and 4000 inject `<script>window.GB_LOGIN_URL = "http://localhost:5000";</script>` into `<head>`, letting the frontend redirect to port 5000 without hardcoding. Controlled by `LOGIN_URL` env var (default `http://localhost:5000`).

### Key Paths
- **Binary:** `target/debug/botserver` (run from `botserver/` directory)
- **Env file:** `botserver/.env`
- **Suite UI:** `botui/ui/suite/` · **Cloud UI:** `botui/ui/cloud/` · **Login UI:** `botui/ui/login/`
- **Cloud API:** botserver `/api/cloud/*` · **Cloud fragment:** botserver `/cloud/partials/sidebar.html`

### BotUI Development Mode
- BotUI serves static HTML/JS/CSS directly from `botui/ui/` — **NO recompilation needed** for frontend changes; refresh the page.
- Only Rust changes in `botui/src/` require rebuild (`cargo build -p botui`).

### Absolute Paths for HTMX Apps
Subdirectory apps (e.g. `/suite/social/social.html`) injected into `/suite/desktop.html` via HTMX: relative paths resolve against `/suite/desktop.html`, NOT the app's directory → 404s (`/suite/social.css`).

**Fix:** ALL resource references in subdirectory app HTMLs MUST use absolute paths starting with `/suite/`:
```html
<!-- ✅ CORRECT — works in both direct nav and HTMX injection -->
<link rel="stylesheet" href="/suite/social/social.css" />
<script src="/suite/social/social.js"></script>

<!-- ❌ WRONG — 404 when injected via desktop launcher -->
<link rel="stylesheet" href="social.css" />
<script src="social.js"></script>
```

---

## Architecture Overview

### Chat Flow

```
User Message (WebSocket) → botserver/src/main_module/ws/handler.rs
  1. WS Connection        → UserSession created, session_id generated, Redis connected
  2. start.bas            → runs ONCE per session (MinIO {bot}.gbai/{bot}.gbdialog/start.bas)
                           ADD_SUGGESTION calls; Redis flag session:{id}:initialized prevents re-run
  3. Message Processing   → stream_response(): IF message_type == 6 → TOOL_EXEC (bypass LLM)
                           ELSE → KB injection (USE_KB) → LLM generate_response()
  4. Tool Execution       → TOOL_EXEC (type 6): direct .ast run via Rhai ScriptService::run()
                           No LLM, no KB — immediate response in chat
  5. LLM Response         → Groq/OpenAI/etc: System + KB + History prompt, streaming via WS chunks
  6. Frontend Display     → botui HTMX/WS: append to #chat-messages, suggestion buttons from
                           Redis suggestions:{bot}:{session}, tool buttons (MessageType 6)
```

### Message Types

| ID | Name | Purpose | LLM Used? |
|----|------|---------|-----------|
| 0 | EXTERNAL | External message | No |
| 1 | USER | User message | Yes |
| 2 | BOT_RESPONSE | Bot response | No |
| 3 | CONTINUE | Continue processing | No |
| 4 | SUGGESTION | Suggestion button | Yes |
| 5 | CONTEXT_CHANGE | Context change | No |
| 6 | **TOOL_EXEC** | **Direct tool execution** | **No — bypasses LLM** |

**TOOL_EXEC (Type 6):** frontend sends `message_type: 6` → backend executes the tool `.ast` directly via Rhai. NO KB injection, NO LLM call. Result appears immediately in chat.

---

## Bot Scripts Architecture

### start.bas — Session Entry Point
- Runs on WebSocket connect; runs again on first user message (blocking, once per session)
- Sets Redis key `session:{session_id}:initialized`; subsequent messages skip it
- Purpose: load suggestions via `ADD_SUGGESTION`, init bot memory, set context

```basic
' start.bas
ADD_SUGGESTION "Check inventory"
ADD_SUGGESTION "Create report"
ADD_SUGGESTION "Send email"
TALK "Hello! I'm your assistant. How can I help?"
```

### tables.bas — Database Schema
**SPECIAL FILE — DO NOT CALL WITH CALL.** Parsed automatically at compile time; defines tables for `sync_bot_tables()`; creates/updates DB schema.

```basic
' tables.bas
BEGIN TABLE customers
    id UUID PRIMARY KEY
    name VARCHAR(255)
    email VARCHAR(255)
    created_at TIMESTAMP
END TABLE

BEGIN TABLE orders
    id UUID PRIMARY KEY
    customer_id UUID REFERENCES customers
    total DECIMAL(10,2)
    status VARCHAR(50)
END TABLE
```

### {tool}.bas — Tool Scripts
- **Source:** the bot's repo `.gbdialog/{tool}.bas` (git-owned bots — ALM repo) → materialized into the work dir `{bot}.gbai/{bot}.gbdialog/` → compiled to `{tool}.ast`. Legacy/local-dev bots keep the same work-dir layout without a repo behind it.
- **Execution:** via `CALL "tool"` or TOOL_EXEC (type 6)

```basic
' detecta.bas - Inventory checker
items = GET FROM inventory WHERE quantity < 10
IF COUNT(items) = 0 THEN
    TALK "All items well stocked!"
ELSE
    response = "Low stock items:\n"
    FOR EACH item IN items
        response = response + "- " + item.name + ": " + item.quantity + "\n"
    NEXT
    TALK response
END IF
```

### CALL Keyword
```basic
CALL "script_name"      ' In-memory procedure or .bas script
CALL "procedure_name"   ' If not in memory, looks for {name}.bas in bot's gbdialog folder
```

### DETECT Keyword
```basic
result = DETECT "folha_salarios"   ' Analyze table for anomalies (requires tables.bas)
                                   ' Calls BotModels API at /api/anomaly/detect
```

---

## Drive & Vault Operations — MANDATORY

**Sources of truth:** for git-owned bots (reform #1501/#1505) `.gbdialog/` scripts and `.gbot/` prompts live in the **bot's ALM repository** — edit them there, commit, push; `git_bot_monitor` materializes the repo into Drive within 15 s. Never "fix" those files by writing straight into Drive: the Drive copy is a materialized artifact and the next sync (or archive pass) clobbers or archives it. Drive (MinIO `{bot}.gbai` buckets) remains the home of runtime state — `.gbkb` knowledge, `.gbdrive` user files and media — manipulated with `mc`, never by touching the local filesystem.

### Drive Bucket Hierarchy — Two Layouts

**Layout 1 — Standalone bot (no org):** bucket `{bot}.gbai` at the top level (e.g. `default.gbai`):
```
{bucket} default.gbai/
├── default.gbdialog/   # scripts: start.bas, {tool}.bas, tables.bas
├── default.gbkb/       # knowledge base docs
├── default.gbot/       # bot config (LLM settings → Vault)
└── default.gbdrive/    # files/reports (optional)
```

**Layout 2 — Org workspace (SaaS):** bucket `{org}.gborg` contains one or more workspace prefixes `{workspace}.gbai/`, each holding **several bots** — every bot is a set of sibling `{bot}.gbot`, `{bot}.gbdialog`, `{bot}.gbkb`, `{bot}.gbdrive` directories (there is NO per-bot bucket):
```
{bucket} cristo.gborg/                 # org (tenant)
└── cristo.gbai/                       # workspace = branch container
    ├── cristo.gbot/                   # bot "cristo" config
    ├── cristo.gbdialog/               # bot "cristo" scripts
    ├── otro.gbot/                     # second bot in the SAME workspace
    └── otro.gbdialog/                 # ...several .gbot inside one .gbai
```

- **Discovery:** `botserver/src/main_module/drive_monitors.rs` scans `.gborg` buckets → lists `.gbai/` prefixes (workspaces) → extracts bot names from object keys `{workspace}.gbai/{bot}.gbdialog/file.bas` → creates `bots` rows (one per bot).
- **Workspace ↔ Drive mismatch:** the SaaS cloud UI (`cloud_workspaces` table) is populated only by signup (`create_cloud_workspace_inner`) or the "New Workspace" API. A `.gborg` uploaded directly via `mc` gets its org + bots auto-created by drive_monitor but has **no `cloud_workspaces` row** (e.g. `salesianos.gborg` exists in Drive but salesianos does not appear in the cloud orgs page). Renaming a workspace row does NOT rename Drive bots.

### Vault credential retrieval (canonical — reuse everywhere)
```bash
source <(grep -E '^VAULT_' ${WORKSPACE}/botserver/.env)   # ONLY VAULT_* variables
export VAULT_ADDR=$VAULT_ADDR
export VAULT_CACERT=${WORKSPACE}/botserver-stack/conf/system/certificates/ca/ca.crt
export VAULT_TOKEN=$(cat /tmp/vault-token-gb 2>/dev/null || echo $VAULT_TOKEN)
VAULT_BIN=${WORKSPACE}/botserver-stack/bin/vault/vault
DRIVE_ACCESSKEY=$($VAULT_BIN kv get -field=accesskey secret/gbo/drive)
DRIVE_SECRET=$($VAULT_BIN kv get -field=secret secret/gbo/drive)
DRIVE_PORT=$($VAULT_BIN kv get -field=port secret/gbo/drive)
/tmp/mc alias set local http://127.0.0.1:${DRIVE_PORT} ${DRIVE_ACCESSKEY} ${DRIVE_SECRET} --api s3v4
```

### Workflow for ANY bot file operation
1. Get credentials from Vault (above) → 2. Configure mc → 3. Pull the file to `/tmp/` for INSPECTION.
   - `.gbkb` / `.gbdrive` content (Drive-owned): edit locally in `/tmp/`, push back with `mc` → drive_monitor reloads.
   - `.gbdialog/` scripts and `.gbot/` prompts of a git-owned bot: make the change in the bot's ALM repo and push — NEVER write script edits back to Drive.

### Common mc operations
```bash
/tmp/mc ls local/                                          # List all bots (each bucket = {bot}.gbai)
/tmp/mc ls local/{bot}.gbai/{bot}.gbdialog/                # Inspect dialog files (read-only for git-owned bots)
/tmp/mc cp local/{bot}.gbai/{bot}.gbdialog/start.bas /tmp/ # Read a bot's start.bas (inspection only)
/tmp/mc ls local/{bot}.gbai/{bot}.gbkb/docs/               # List KB documents
/tmp/mc cp /tmp/document.pdf local/{bot}.gbai/{bot}.gbkb/docs/        # Upload KB doc (Drive-owned content)
/tmp/mc ls local/{bot}.gbai/{bot}.gbdrive/                 # Inspect user files/media (Drive-owned)
/tmp/mc mb local/{bot}.gbai && /tmp/mc cp --recursive botserver-stack/data/system/work/{bot}.gbai/ local/{bot}.gbai/  # Upload bot
```

### LLM Configuration — Vault (single source of truth)
**Location (ALL LLM settings):** Vault per-bot path `secret/gbo/{org_id}/{branch_id}/{bot_id}`
**Global fallback:** `secret/gbo/llm` (read at boot for the base provider)

| Field | Where it lives | Description |
|-------|----------------|-------------|
| `llm-url` | **Vault** | Full URL for chat completions |
| `llm-server` | **Vault** | Base server URL |
| `llm-key` | **Vault** | API key for the LLM provider |
| `llm-model` | **Vault** | Model identifier |
| `llm-provider` | **Vault** | Provider type |
| `system-prompt` | **Vault** | Bot personality/instructions |
| `history-limit` | **Vault** | Conversation history turns |

**How it works:** 1) All LLM settings are read from Vault via `ConfigManager::get_config()` (per-bot path → nil path → global `secret/gbo/llm`). 2) Falls back to env vars `LLM_URL`/`LLM_MODEL`/`LLM_KEY` only when Vault has no value. 3) Provider auto-detected from URL pattern.

**Update LLM settings (Vault):**
```bash
export VAULT_ADDR=<vault-addr> VAULT_SKIP_VERIFY=true VAULT_TOKEN=<root-token>
# resolve org/branch from: SELECT org_id, branch_id FROM bots WHERE name = '<bot>';
vault kv put secret/gbo/<org_id>/<branch_id>/<bot_id> \
  llm-url=<chat-completions-url> llm-model=<model> llm-key=<api-key>
# Restart botserver to apply (provider is built at boot).
```

**Set global fallback (used when a bot has no per-bot entry):**
```bash
vault kv put secret/gbo/llm url=<chat-completions-url> model=<model> openai_key=<api-key>
```

---

## Chrome CDP Testing (9222)

### Start Chrome (reuse profile for persistent sessions)
```bash
# If not running:
cp -a ~/.config/google-chrome /tmp/chrome-persistent-profile   # preserves WhatsApp/cookies/logins
export DISPLAY=:1
google-chrome --no-sandbox --disable-gpu --remote-debugging-port=9222 \
  --remote-allow-origins=* --user-data-dir=/tmp/chrome-persistent-profile --start-maximized &
```
Check first: `ps aux | grep "chrome.*remote-debugging-port=9222" | grep -v grep`

### Workflow
1. **Each use case = a separate tab** (max 10). NEVER reuse tabs. Open via CDP:
   `python3 -c "import requests; requests.put('http://127.0.0.1:9222/json/new?URL')"` or manually in Chrome.
2. Navigate: `http://localhost:3000/{bot}` (bot chat) or `http://localhost:4000/cloud` (SaaS cloud).
3. Interact: type messages, click suggestion buttons, execute tools.
4. 🚨 **NEVER close the browser** — tabs are trace evidence. Close only when explicitly requested.
5. Screenshots at `/tmp/{bot}_case{N}_{desc}.png`.

### Fragmented CDP Control (MANDATORY)
❌ NEVER generate long, all-in-one automation scripts that batch navigation + login + clicks + assertions in a single run. They hide progress, fail opaquely mid-way, and orphan state (e.g. a crashed step closed a tab and every later step died silently).

✅ **One script = ONE step.** Each step:
1. Does exactly one action (open tab, OR refresh token, OR click a button, OR read UI state, OR screenshot)
2. `print()`s what it found (URL, visible text, API status)
3. **STOPS** — results are reported before the next step is written

```python
# ❌ WRONG — 200-line script: goto → login → SSO → click folder → assert → screenshot
# ✅ RIGHT — /tmp/step1_open.py opens the tab and prints the landing state; DONE.
#           Then inspect output, decide next single step, write /tmp/step2_*.py
```

Conventions:
- Scripts live in `/tmp/` named `{topic}_step{N}_{action}.py` (e.g. `beiner_drive_step4_sso.py`)
- Re-locate tabs/pages at the START of each script (never cache page handles across steps — a previous step may have closed or crashed the tab)
- If a step fails, diagnose from its printed state before writing the next step; never retry a batch blindly
- Between steps, keep the todo list current (see Non-Negotiable Rules → Todos)

**Hard limits (learned 2026-09-30 — long scripts froze the browser and wasted the session):**
- ❌ NEVER put a polling loop (`while`/`for` + `asyncio.sleep`) inside a script — a long script looks like a hung browser; instead read state with repeated one-shot scripts
- ❌ NEVER `asyncio.sleep()` longer than ~5 s in an action script — if the server is slow (prod `/api/files/*` calls take 16–32 s), run another small read step later
- ❌ NEVER batch two actions in one file (reload + read = two steps, two scripts)
- ✅ Keep every script under ~30 lines: locate the page by URL → one action OR one read → `print()` compact state (JSON) → exit
- ✅ Timebox the run (~60 s tool timeout); if it exceeds, kill it, split the step, keep the browser open

### Suite Apps — ALWAYS Open Inside Desktop (NEVER Direct URL)
**❌ NEVER open a suite app page directly** (`/suite/drive/drive.html`, `/suite/chat/chat.html`, etc.) — they are HTMX fragments requiring the desktop shell (`desktop.html`) to bootstrap JS modules, security context, window manager. Direct URL = broken empty shell.

**✅ Correct flow — navigate to the desktop route, never the HTML file:**
- `/drive` → desktop shell detects app "drive" → HTMX loads `/suite/drive/drive.html` into content area
- `/chat/<bot>` → desktop shell → `/suite/chat/chat.html`
- `/tasks` → `/suite/tasks/tasks.html` · `/social` → `/suite/social/social.html`

**Bot context comes from the HOST, not a query param:** test a bot on its own platform subdomain — `https://{bot}.{platform}/drive` (e.g. `https://beiner.generalbots.org/drive`). On another bot's host (`https://pragmatismo.generalbots.org/drive?bot=beiner`) the shell derives `window.__INITIAL_BOT_NAME__` from the Host header (`pragmatismo`) and the `?bot=` param is ignored — Drive then probes the wrong buckets and never renders.

**Why:** Drive app has 5 top-level tabs (Bots, My Files, Shared, Public, Root). Direct URL skips the `01_state.js` → `99_init.js` chain — no tab bar, no API calls, broken shell.

**Verification:** after login/SSO hop to `/drive`, the desktop shell must be visible (window manager, taskbar, app icons, title bar). Drive appears as a **window inside the desktop**, NOT a standalone full-page view.

**Test in Chrome:** open `http://localhost:3000/`, log in (or SSO redirect), type `/drive` in the desktop launcher; or CDP: `http://localhost:9222/json/new?http://localhost:3000/drive`.

### Three Pillars of Bot Testing
- **Drive (mc)** — all bot files come from MinIO, manipulated via `mc` with Vault credentials
- **Vault (.env)** — credentials ALWAYS from Vault; only `VAULT_*` variables may be in `.env`
- **Chrome CDP (9222)** — visual debugging ALWAYS via remote Chrome, separate tab per use case. ❌ NEVER via direct WebSocket

---

## BASIC Keywords Reference

### Language Guidelines
Use formal language in comments and documentation — no slang, neologisms, or informal expressions; maintain professional tone.

### Data Operations
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `GET` | `GET FROM {table} WHERE {condition}` | Query database records |
| `SET` | `SET {variable} = {value}` | Set variable value |
| `SAVE` | `SAVE {data} TO {table}` | Insert/update database record |
| `FIND` | `FIND {value} IN {table}` | Search for specific value |
| `FIRST`/`LAST` | `FIRST({array})` / `LAST({array})` | Get first/last element |
| `COUNT` | `COUNT({array})` | Count elements |
| `FORMAT` | `FORMAT "{template}", var1, var2` | Format string with variables |

### Communication
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `TALK` | `TALK "{message}"` | Respond to user (`TALK "Line 1\nLine 2"` multi-line; supports `+` concat) |
| `HEAR` | `HEAR "{phrase}" AS {variable}` | Listen for user input (used in voice/chat triggered tools) |
| `SEND MAIL` | `SEND MAIL TO "{email}" WITH subject, body` | Send email |
| `SEND TEMPLATE` | `SEND TEMPLATE "{name}" TO "{email}"` | Send email template |
| `SEND SMS` | `SEND SMS TO "{phone}" MESSAGE "{text}"` | Send SMS |

### File Operations
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `CREATE FILE` | `CREATE FILE "{path}" WITH {content}` | Create file in .gbdrive |
| `READ FILE` | `READ FILE "{path}"` | Read file content |
| `WRITE FILE` | `WRITE FILE "{path}" WITH {content}` | Write to file |
| `DELETE FILE` / `COPY FILE` / `MOVE FILE` / `LIST FILES` | `DELETE|COPY|MOVE FILE ...` / `LIST FILES "{path}"` | File management |
| `UPLOAD` / `DOWNLOAD` | `UPLOAD {data} TO "{path}"` / `DOWNLOAD "{url}" TO "{path}"` | Upload to MinIO / download |

### HTTP Operations
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `GET HTTP` / `POST HTTP` / `PUT HTTP` / `DELETE HTTP` | `GET HTTP "{url}"` etc. | HTTP requests |
| `WEBHOOK` | `WEBHOOK "{url}" WITH {data}` | Send webhook |

### AI/LLM Operations
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `LLM` | `LLM "{prompt}"` | Call LLM with prompt |
| `USE KB` | `USE KB "{knowledge_base}"` | Use knowledge base for context |
| `CLEAR KB` | `CLEAR KB` | Clear KB context |
| `USE TOOL` / `CLEAR TOOLS` | `USE TOOL "{tool}"` / `CLEAR TOOLS` | Enable/disable external tools |
| `USE WEBSITE` | `USE WEBSITE "{url}"` | Scrape website for context |

**USE KB flow:** `USE KB "manual"` → bot searches `.gbkb/` → chunks text, creates embeddings → queries Qdrant for relevant chunks → injects into LLM prompt as context → user question answered with KB context.

### Task & Scheduling
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `CREATE_TASK` | `CREATE_TASK "{title}", "{assignee}", "{due}", {project}` | Create task |
| `WAIT` | `WAIT {seconds}` | Pause execution |
| `ON` / `ON EMAIL` / `ON CHANGE` | `ON "{event}" DO {action}` etc. | Event handlers (`ON EMAIL FROM "@company.com" DO CALL "process_email"`) |

### Bot & Memory
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `SET BOT MEMORY` / `GET BOT MEMORY` | `SET BOT MEMORY "{key}" = {value}` | Bot-level memory (persists across sessions) |
| `REMEMBER` / `RECALL` | `REMEMBER "{key}" = {value}` | Session-level memory |
| `SET CONTEXT` | `SET CONTEXT "{key}" = {value}` | Conversation context |
| `ADD_SUGGESTION` | `ADD_SUGGESTION "{text}"` | Add quick-reply button (in start.bas). Deduped via Redis SADD, key `suggestions:{bot_id}:{session_id}`, read with SMEMBERS |
| `CLEAR SUGGESTIONS` | `CLEAR SUGGESTIONS` | Clear suggestions |

### User & Session
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `SET USER` | `SET USER "{property}" = {value}` | Update user property |
| `TRANSFER TO HUMAN` | `TRANSFER TO HUMAN` | Escalate to human agent |
| `ADD_MEMBER` | `ADD_MEMBER "{group}", "{email}", "{role}"` | Add user to group |

### Documents & Content
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `CREATE DRAFT` | `CREATE DRAFT "{title}" WITH {content}` | Create document draft |
| `CREATE SITE` | `CREATE SITE "{name}" WITH {config}` | Create website |
| `SAVE FROM UNSTRUCTURED` | `SAVE FROM UNSTRUCTURED {data} TO {table}` | Parse and save data |

### Multi-Bot Operations
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `ADD BOT` / `REMOVE BOT` / `LIST BOTS` | `ADD BOT "{name}" WITH TRIGGER "{phrase}"` | Sub-bot management |
| `DELEGATE TO` | `DELEGATE TO "{bot}"` | Delegate to another bot |
| `SEND TO BOT` | `SEND TO BOT "{name}" MESSAGE "{msg}"` | Inter-bot message |
| `BROADCAST MESSAGE` | `BROADCAST MESSAGE "{msg}"` | Broadcast to all bots |

### Social Media
| Keyword | Syntax | Description |
|---------|--------|-------------|
| `POST TO SOCIAL` | `POST TO SOCIAL "{platform}" MESSAGE "{text}"` | Social media post |
| `GET SOCIAL FEED` | `GET SOCIAL FEED "{platform}"` | Get social feed |

### Control Flow
```basic
IF condition THEN ... ELSE ... END IF
FOR EACH item IN collection ... NEXT
SWITCH var CASE val ... DEFAULT ... END SWITCH
PRINT {value}     ' Debug output
```

### Built-in Variables
| Variable | Description | Example |
|----------|-------------|---------|
| `TODAY` | Current date | `IF created_at == TODAY THEN` |
| `NOW` | Current datetime | `SET last_seen = NOW` |
| `USER` | Current user object | `USER.email`, `USER.id` |
| `SESSION` | Current session object | `SESSION.id` |
| `BOT` | Current bot object | `BOT.name`, `BOT.id` |

### AutoTask System
AI-driven task execution: 1) Analyze user intent ("Send email to all customers") → 2) Plan execution steps → 3) Generate BASIC scripts from available keywords → 4) Execute immediately or schedule.

**File locations:**
```
.gbdrive/
├── reports/           # Generated reports
├── documents/         # Created documents
├── exports/           # Data exports
└── apps/{appname}/    # HTMX apps (synced to SITES_ROOT)
.gbdialog/
├── schedulers/        # Scheduled jobs (cron-based)
├── tools/             # Voice/chat triggered tools
└── handlers/          # Event handlers
```

---

## LLM Navigation Guide

`/opt/gbo/data` also holds bots. **For LLMs analyzing this codebase:**
0. Bots are in drive; each bucket is a bot. Respect LOAD_ONLY.
1. Start with [Component Dependency Graph](../README.md#-component-dependency-graph) in README
2. Review [Module Responsibility Matrix](../README.md#-module-responsibility-matrix)
3. Study [Data Flow Patterns](../README.md#-data-flow-patterns)
4. Reference [Common Architectural Patterns](../README.md#-common-architectural-patterns) before changes
5. Check [Security Directives](#-security-directives---mandatory) — violations are blocking
6. Follow [Mandatory Code Patterns](#-mandatory-code-patterns) — consistency is mandatory

---

## Reset Process Notes

- **Purpose:** reset.sh cleans and restarts the dev environment; bootstrap takes 3-5 min (Vault, PostgreSQL, Valkey, MinIO, Zitadel, LLM)
- **Timeout risk:** script can timeout on "Step 3/4: Waiting for BotServer to bootstrap"
  - Zitadel not ready within 60s → admin user creation fails; script waits indefinitely
  - **Solution:** check `botserver.log` for "Bootstrap process completed!"
- **Zitadel Not Ready:** "Bootstrap check failed" → directory service may need >60s; admin creation deferred; services still start
- **Services Exit After Start:** check logs for "dispatch failure"; check Vault cert errors: `tls: failed to verify certificate: x509`

**Manual service management:**
```bash
ps aux | grep -E "(botserver|botui)" | grep -v grep
curl http://localhost:8080/health
tail -f botserver.log botui.log
./restart.sh
```

**Reset verification:** ✅ PostgreSQL 5432 · ✅ Valkey 6379 · ✅ BotServer 8080 · ✅ BotUI 3000/4000/5000 · ✅ no errors in botserver.log

### Dev Dependencies (Hot Testing)
```bash
# start.bas must use double quotes (BASIC syntax); should show: TALK "Hello from default bot"
cat /opt/gbo/data/default.gbai/default.gbdialog/start.bas
# Verify tool registration (expected: Registered tool 'start' in database)
grep -E "Registered tool|Compiled.*start" botserver.log
# Clean reset from scratch
killall -9 botserver vault postgres valkey minio qdrant zitadel 2>/dev/null; sleep 2
rm -rf botserver-stack/ botserver/botserver-stack/ .env botserver/.env botserver.log 2>/dev/null
BOTMODELS_HOST="http://localhost:8085" BOTMODELS_API_KEY="starter" RUST_LOG=info \
  nohup ./target/debug/botserver --noconsole > botserver.log 2>&1 &
```

### Staging Environment (STAGE-GBO)
- `chat.stage.pragmatismo.com.br` uses `10.0.3.x` subnet for container IPs (e.g. `10.0.3.10` system container)
- Route testing via host gateway `10.0.0.1` or hit container IPs directly inside the staging host
- Do NOT confuse staging IP ranges (`10.0.3.x`) with production ranges

---

## Security Directives — MANDATORY

### 1. Error Handling — NO PANICS IN PRODUCTION
`botserver` serves thousands of simultaneous sessions 24/7; any `panic!` crashes the process and interrupts all users. Every error path must propagate via `Result` or be handled locally. `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` are strictly forbidden outside tests.

```rust
// ❌ FORBIDDEN (aborts process)
let raw = std::fs::read_to_string(path).unwrap();
let cfg: Config = serde_json::from_str(&raw).expect("parse");
POOL.get().unwrap();
todo!("not implemented");

// ✅ REQUIRED (propagate via ?, or handle locally with log)
fn load_config(path: &str) -> Result<Config, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(path.into(), e))?;
    Ok(serde_json::from_str(&raw).map_err(ConfigError::Parse)?)
}
fn load_or_default(path: &str) -> Config {
    match load_config(path) {
        Ok(cfg) => cfg,
        Err(e) => { log::error!("config load failed for {path}: {e}"); Config::default() }
    }
}
```

**Quick translation table:**

| Forbidden pattern | Mandatory replacement |
|-------------------|-----------------------|
| `value.unwrap()` | `value?` (in fn returning `Result`) or `value.ok_or_else(|| Error::X)?` |
| `value.expect("msg")` | `value.context("msg")?` (anyhow) or `value.map_err(|e| Error::X(e))?` |
| `panic!("...")` | `return Err(Error::X.into());` |
| `todo!()` / `unimplemented!()` | real body, or `unimplemented!()` documented in `#[cfg(test)]`, or `return Err(Error::NotImplemented.into());` |
| `if let Some(v) = x { ... }` loose | `match x { Some(v) => ..., None => return Err(...) }` |
| `match x { Ok(v) => v, Err(_) => default }` silent | `match x { Ok(v) => v, Err(e) => { log::error!(...); default } }` |

### 2. Command Execution — USE SafeCommand
```rust
// ❌ Command::new("some_command").arg(user_input).output()
// ✅
use crate::security::command_guard::SafeCommand;
SafeCommand::new("allowed_command")?.arg("safe_arg")?.execute()
```

### 3. Error Responses — USE ErrorSanitizer
```rust
// ❌ Json(json!({ "error": e.to_string() }))
// ✅
use crate::security::error_sanitizer::log_and_sanitize;
let sanitized = log_and_sanitize(&e, "context", None);
(StatusCode::INTERNAL_SERVER_ERROR, sanitized)
```

### 4. SQL — USE sql_guard
```rust
// ❌ format!("SELECT * FROM {}", user_table)
// ✅
use crate::security::sql_guard::{sanitize_identifier, validate_table_name};
let safe_table = sanitize_identifier(&user_table);
validate_table_name(&safe_table)?;
```

### 5. Rate Limiting (IMP-07)
- Defaults: General 100 req/s (global) · Auth 10 req/s (login endpoints) · API 50 req/s (per token)
- MUST use `governor` crate; per-IP and per-User tracking; WebSocket message rate limits (e.g. 10 msgs/s)

### 6. CSRF Protection (IMP-08)
- ALL state-changing endpoints (POST, PUT, DELETE, PATCH) MUST require a CSRF token
- Use `tower_csrf` or similar; token bound to user session; Double-Submit Cookie or Header-based
- **Exemptions:** API endpoints using Bearer Token auth (stateless)

### 7. Security Headers (IMP-09) — on ALL responses
`Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none';` · `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload` · `X-Frame-Options: DENY` or `SAMEORIGIN` · `X-Content-Type-Options: nosniff` · `Referrer-Policy: strict-origin-when-cross-origin` · `Permissions-Policy: geolocation=(), microphone=(), camera=()`

### 8. Dependency Management (IMP-10)
- Application crates (`botserver`, `botui`) MUST track `Cargo.lock`; library crates (`botlib`) MUST NOT
- Critical deps (crypto, security) MUST use exact versions (e.g. `=1.0.1`); regular MAY use caret
- Run `cargo audit` weekly; update deps only via PR with testing

---

## Mandatory Code Patterns

```rust
impl MyStruct { fn new() -> Self { Self { } } }        // Self, not MyStruct
#[derive(PartialEq, Eq)]                              // Always both
struct MyStruct { }
format!("Hello {name}")                               // Inline args, not format!("{}", name)
match x { A | B => do_thing(), C => other() }         // Combine identical arms
```

## Absolute Prohibitions

- NEVER search the `/target` folder — it is binary compiled
- ❌ NEVER build in release mode / use `--release` — ONLY debug builds
- ❌ NEVER run `cargo build` — use `cargo check` for syntax verification
- ❌ NEVER run `cargo check` synchronously — always `nohup cargo check > /tmp/<crate>_check.log 2>&1 &`
- ❌ NEVER compile directly for production / copy binaries manually — ALWAYS push + CI/CD
- ❌ NEVER SSH into bot container to deploy binaries — CI workflow handles build, transfer, restart via alm-ci SSH
- ✅ ALWAYS push code to ALM → CI builds → CI deploys to bot container via SSH from alm-ci
- ✅ CI deploy path: alm-ci builds at `/opt/gbo/data/botserver/target/debug/botserver` → tar+gzip via SSH → `/opt/gbo/bin/botserver` on bot container → restart
- ❌ NEVER change git branches without explicit user approval
- ❌ NEVER use `panic!()`, `todo!()`, `unimplemented!()`
- ❌ NEVER use `Command::new()` directly — use `SafeCommand`
- ❌ NEVER return raw error strings to HTTP clients — use `log_and_sanitize`
- ❌ NEVER use `#[allow()]` — FIX the code; never add lint exceptions to `Cargo.toml`
- ❌ NEVER use `_` prefix for unused variables — DELETE or USE them; no unused imports or dead code
- ❌ NEVER use CDN links — all assets must be local
- ❌ NEVER create `.md` files without checking `botbook/` first
- ❌ NEVER comment out code — FIX it or DELETE it entirely

---

## Vibe Per-Project Databases (#1386)

Every Vibe project kind owns a **dedicated PostgreSQL database per environment** — dev work can never read or write production data:

| Kind | Production DB | Dev/test DB | Injection |
|------|---------------|-------------|-----------|
| **bot** | `bot_{branch}_{bot}` (lazy, `botcore/bot_database.rs`) | `bot_{branch}_{bot}_dev` via the `{bot}-dev` twin bot (automatic — the twin is a normal bot row) | per-bot pool |
| **apps** (node/python) | `app_{branch}_{slug}` | `app_{branch}_{slug}_dev` | `DATABASE_URL` in the VM `vibe-app.service` / proxy python unit (via persistent `/etc/gb-vibe/{unit}.env` EnvironmentFile). **ALM/CI-CD deploys:** the publish flow resolves the URL once (`publish.rs`) and carries it through `DeploymentRequest.database_url` → gateway → `app.service` (`gateway_runtime.rs` injects `Environment=DATABASE_URL=`); the gateway never invents one. |
| **website** | — (static) | — (static) | none |

- Naming: `botvibe/src/project_db.rs` (`ensure_project_database`, `drop_project_databases`); `_dev` suffix is **reserved** — manually created bots/apps may not end in `-dev` (`BotDatabaseManager::is_reserved_dev_bot_name`).
- Promotion copies **code only**; databases are never touched by promote/rollback.
- The DB dialog (`botui/ui/suite/vibe/vibe-dialog-db.js`) has a **dev/prod selector** sending `X-Db-Env: dev|production`; `botdatabase` handlers resolve the `_dev` twin database (created on demand) when `dev` is selected.
- Project delete/eviction drops both databases (`WITH (FORCE)`).

---

## File Size Limits — MANDATORY

**NEVER let a single file exceed 450 lines — split proactively at 350 lines.** When growing beyond:
1. Identify logical groups → 2. Create subdirectory module (`handlers/` Rust, `modules/` JS) → 3. Split by responsibility (`types.rs`, `handlers.rs`, `operations.rs`, `utils.rs`, `mod.rs`) → 4. Keep files focused → 5. Update `mod.rs` re-exports.

### JS Frontend Module Pattern (`botui/ui/suite/<app>/`)
```
<app>.html          # Loads modules via <script> tags
<app>.js.orig       # Original monolithic file (backup)
modules/
├── 01_state.js     # Config, state, constants
├── 02_render.js    # Rendering logic
├── 03_events.js    # Event handlers
└── 99_init.js      # Only DOMContentLoaded/init() call
```
- Plain `<script>` files (NOT ES modules) — functions globally visible via load order
- Ordered from lowest number to `99_init.js`; each module MUST start with `"use strict";`
- Naming: `{order}_{firstFunctionName}.js`

---

## Error Fixing Workflow

### Mode 1: OFFLINE Batch Fix (PREFERRED)
1. Read ENTIRE error list first → 2. Group errors by file → 3. For EACH file: view → fix ALL errors → write once → 4. Move to next file → 5. Repeat until all addressed → 6. **ONLY THEN** verify with build/diagnostics. **NEVER run cargo during fixing.**

### Mode 2: Interactive Loop
```
LOOP UNTIL (0 warnings AND 0 errors):
  Run diagnostics → pick file → read entire file → fix ALL issues → write once → verify → CONTINUE
```

### Streaming Build Rule
Do NOT wait for `cargo` to finish. As soon as the first errors appear, cancel the build, fix those errors immediately, re-run.

### Parallel & Non-Blocking Execution Philosophy
1. **NEVER WAIT** — long-running tools (cargo check/clippy/builds) run in background via `nohup`
2. **ALWAYS PARALLEL** — launch multiple independent checks simultaneously
3. **KEEP THINKING** — while processes run, analyze code, plan fixes, read files, write edits
4. **POLL LATER** — check background results when convenient, not immediately
5. **NEVER IDLE** — if a tool is running, start another task
6. **NEVER STOP THE LOOP** — drive autonomously until 0 warnings/0 errors or a genuine blocker

```bash
nohup cargo check -p botcore > /tmp/botcore_check.log 2>&1 &
nohup cargo check -p botserver > /tmp/botserver_check.log 2>&1 &
nohup cargo check -p botlib > /tmp/botlib_check.log 2>&1 &
# ... keep analyzing; later:
tail -50 /tmp/botcore_check.log
ps aux | grep 'cargo check' | grep -v grep
```
- ❌ Never run `cargo check` synchronously · ❌ never run only one check when several crates need verification
- ❌ never idle-wait · ❌ never stop mid-loop to ask "should I continue?"
- ❌ never fix errors one-by-one when a Python script can batch-fix (5+ same-pattern errors)
- ✅ logs to `/tmp/` only · ✅ check running procs before launching (`ps aux | grep cargo`)
- ✅ kill stale procs before re-launching (`pkill -f "cargo check -p <crate>"`) · ✅ continue analysis while running

### Python Batch-Fix Scripts (5+ errors sharing a pattern)
Script: reads `/tmp/<crate>_check.log` → parses file:line:col → applies regex fixes per file → writes all at once → reports. Save to `/tmp/fix_*.py`, run `python3 /tmp/fix_*.py`.

**LLM-enabled scripts:** for semantic fixes use `openai` package with `DEV_LLM_URL`/`DEV_LLM_KEY` from `.env` (DEV-only keys, never committed). Script sends error + code context, LLM returns fixed code.

### Memory Management (process "Killed")
```bash
pkill -9 cargo; pkill -9 rustc; pkill -9 botserver
CARGO_BUILD_JOBS=1 cargo check -p botserver 2>&1 | tail -200
```

---

## Automatic Bot Testing Workflow

**When user says "test bot" — do this autonomously:**
1. **Ask** "What bot would you like to test today?" (do NOT assume a bot name)
2. **Get Drive credentials from Vault** (see [Drive & Vault Operations](#-drive--vault-operations--mandatory))
3. **Run restart.sh** — `nohup ./restart.sh > /tmp/restart.log 2>&1 &`
4. **Wait for bootstrap** — poll `curl -s http://localhost:8080/health` until 200 (up to 5 min)
5. **Find the bot** — `/tmp/mc ls local/` (each bucket = `{bot}.gbai`)
6. **If bot not in drive, ask user** — do NOT copy from work dir. Ask: "Where can I get a copy of the .gbai to work on?"
7. **Verify bot loaded** — check botserver logs for `[drive_monitor]` confirming bot sync
8. **Start Chrome CDP** if not running (see [Chrome CDP Testing](#-chrome-cdp-testing-9222))
9. **🚨 NEVER CLOSE THE BROWSER** — each open tab is a use case for user inspection
10. **Open tabs via Playwright CDP** (connects to already-open Chrome on 9222; tabs persist after script):
    ```python
    from playwright.async_api import async_playwright
    async with async_playwright() as p:
        browser = await p.chromium.connect_over_cdp("http://localhost:9222")
        page = await browser.contexts[0].new_page()
        await page.goto('http://localhost:4000/cloud/signup')
        # ... interact ... DO NOT close the browser
    ```
11. **Test 3 use cases in separate tabs:**
    - Case 1 (Greeting): Send "Hello", check welcome TALK + suggestion buttons
    - Case 2 (Main service): message about main service, check data collection flow
    - Case 3 (Second service or pending items): second service or list pending items
12. **Verify responses** — `page.evaluate()` to capture last `.message.bot .bot-message`
13. **Screenshots** → `/tmp/{bot}_case{N}_{before|after}.png`
14. **Report** — visual evidence + summary per case: messages sent, responses, suggestions

### Interactive Chat Testing — ONE STEP AT A TIME
**CRITICAL: NEVER batch-send messages.** Send one message, read the bot's response, analyze it, then decide the next message based on what the bot asks for.

```python
# STEP 1 (single execution): send ONE message, print response
await page.fill("#messageInput", "quero agendar um batizado")
await page.keyboard.press("Enter")
await asyncio.sleep(12)
r = await page.evaluate("""() => {
    const c = document.getElementById('messages');
    if (!c) return 'NONE';
    const bots = Array.from(c.children).filter(el => el.className.includes('bot'));
    return bots.length ? bots[bots.length-1].textContent.trim().substring(0,600) : 'NONE';
}""")
print(r)
```
Then analyze the output, type the next field the bot asks for, repeat until the tool is called or flow completes.
**NEVER:** ❌ pre-define all fields in a list/loop · ❌ send multiple fields without reading each response · ❌ assume what the bot will ask next — always read the actual response first.

**Key commands:**
```bash
curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/health   # health check
# Vault credentials (always first) — see canonical block in Drive & Vault Operations
# Check botserver logs for errors
grep -E "ERROR|WARN|drive_monitor" botserver.log | tail -20
```

### Vibe Testing — REUSE the Same Project (VM disk space)
Each Vibe project raises its own Incus VM (`incus launch images:ubuntu/24.04`, see `botserver/crates/botvibe/src/vm_incus.rs`). Every new project spawns a fresh Ubuntu container that consumes host disk — repeated "New Project" runs during testing stack up VMs and can exhaust the disk (this machine hit **100% full**). **When testing Vibe, reuse an existing project instead of creating a new one.** Only create a new project when a clean, isolated VM is genuinely required, and clean up after: `incus delete --force <container>` (or reuse the VM). Before spawning more VMs check `df -h /` and `incus list`.

---

## Cloud SaaS Product Architecture (CRM + Default Bot)

Products live in the `botproducts` crate, NOT in CRM. They share the `org_id`/`bot_id`/`branch_id` scope but are separate domains with no FK between them.

**Products are scoped by `branch_id`.** `get_bot_context()` at `botproducts/src/lib.rs:33` resolves to `branch_id = Uuid::nil()` in global SaaS admin mode, or the org's real branch after signup.

### Feature Gates & Dependency Chain
| Feature | Enables | Effect |
|---------|---------|--------|
| `people` | CRM (contacts, tickets, leads) | `botcrm` crate |
| `billing` | Product CRUD routes | `botproducts` crate |
| `saas` | seeding + cloud API + subscriptions | includes `billing` + `botproducts` |

Chain: `saas` → `billing` → `botproducts`

### Product Seeding (idempotent)
`botproducts::seed::seed_default_products(conn, branch_id)` triggered at:
1. **Server init** (`init.rs:269`): once with `Uuid::nil()` (global catalog — visible to all orgs before signup)
2. **Org signup** (`botcloud/api.rs:376`): with the new org's `branch_id` (dedicated product scope)

Seeded products (all `stock_quantity: -1` / unlimited):

| Category | SKUs | product_type |
|----------|------|-------------|
| Plans | `free`($0), `shared`($3.99), `private-cloud`(custom) | `plan` |
| VMs | `vps-small`($9.99), `vps-medium`($19.99), `vps-large`($39.99) | `infrastructure` |
| GPU | `gpu-basic`($39.99), `gpu-advanced`($99.99) | `infrastructure` |
| Storage | `storage-50gb`($9.99), `storage-200gb`($29.99), `storage-1tb`($99.99) | `infrastructure` |
| Comms | `number-local`($5.99), `number-tollfree`($9.99), `domain-com`($21.99/yr), `domain-org`($19.99/yr) | `communication` |
| LLM Tokens | `llm-1m`($9.99), `llm-10m`($79.99) | `llm-tokens` |

### `get_default_bot` Resolver
Determines which bot's product scope to use. Default bot = SaaS backend (super admin logs in via port 3000, sees CRM/products/clients/billing; Store/Plans pages on port 4000 read the same scope).

- **SaaS active** (`Some(|_c| (nil, "default"))`): queries use `branch_id = Uuid::nil()` — seeded products globally visible
- **SaaS inactive** (`None`): **known bug** — handlers return `None` prematurely → empty grids

All crates MUST use `Some(...)` when the corresponding feature is active:

| Crate | Feature | get_default_bot | File/line |
|-------|---------|-----------------|-----------|
| `botproducts` | `billing` | `Some(\|_c\| (nil, "default"))` | `server.rs:379` |
| `bottickets` | `tickets` | `Some(\|_c\| (nil, "default"))` | `server.rs:387` |
| `botpeople` | `people` | `Some(\|_c\| (nil, "default"))` | `server.rs:397` |
| `botattendant` | `attendant` | `Some(\|_c\| (nil, "default"))` | `server.rs:409` |
| `botworkspaces` | `workspaces` | `Some(\|_c\| (nil, "default"))` | `server.rs:371` |

**⚠️ IMPORTANT:** Do not confuse with `get_default_bot` in `botcloud` (signup) — there the closure returns the first active bot (`query_first_bot`) for per-org scoping. In suite/admin we use `(nil, "default")` for the global default bot scope.

**Bug history:** ProductsState had `get_default_bot: None` for weeks after module refactoring (jun/2026); other crates correctly used `Some(|_c| (nil, "default"))`. Fix: same pattern in `products_routes` at `server.rs:379-380`. Implementation: `botproducts/src/lib.rs:33-45`.

### Product Routes
- **REST API** (`/api/products/*`): CRUD for items, services, categories, price-lists, stock, stats, low-stock
- **HTMX fragments** (`/api/ui/products/*`): grids/tables for items, services, pricelists, stats, search

### Cloud Pages That Display Products
| Page | Path | Shows |
|------|------|-------|
| Store | `botui/ui/cloud/store.html` | Full catalog (hardcoded + API overlay) |
| Plans | `botui/ui/cloud/plans.html` | Plan grid from API |
| Offers | `botui/ui/cloud/offers.html` | Bundles |
| Dashboard | `botui/ui/cloud/dashboard.html` | Current plan + usage |
| Signup | `botui/ui/login/signup.html` (port 5000) | Plan selector |

### Important LLM Rules
- **NEVER create a separate admin products HTML page** — products show through existing Store/Plans/Dashboard pages
- **Products are NOT CRM entities** — same scope, separate tables, no FK relationship
- **Seeding is idempotent** — checks `products::table.filter(branch_id)` before inserting
- **Default bot with nil UUID** is a global fallback; in production SaaS, `query_first_bot` finds the real active bot

### Cloud Domain Architecture
| Domain | Port | Serves | Notes |
|--------|------|--------|-------|
| `pragmatismo.com.br` | Caddy → 4000 | Landing + Cloud store/Plans | Static landing + cloud UI via botui |
| `cloud.pragmatismo.com.br` | Caddy → 4000 | Cloud dashboard, store, plans | Same botui cloud server, different proxy route |
| `chat.pragmatismo.com.br` | Caddy → 8080 | Bot API, WebSocket | Proxied directly to botserver |
| `login.pragmatismo.com.br` | Caddy → 5000 | Login, signup | Botui login server |

**Store page** (`/store` or `/cloud/store`): served by botui on port 4000 (`botui/ui/cloud/store.html`); loads products via `GET /api/catalog/products` (public). If the API is offline, `calc.js` uses local estimates (fallback).

> **Important:** Products are seeded by `branch_id`. Global catalog (SAAS) uses `branch_id = nil`. On signup a new branch is created and products seeded for it — each organization sees its own product set.

---

## Domain Management — Platform Subdomains + Custom DNS → Bot Mapping

**Managed in cloud manager UI** (`/domains` on port 4000, admin-only via super admin check). Associates hostnames (e.g. `chat.pragmatismo.com.br`) with specific bots.

### Subdomain Mode (SaaS publishing default — parameterized)
Every published bot is reachable at `{botname}.{GB_PLATFORM_DOMAIN}` (default `generalbots.org`) — no `bot_domains` row, no custom domain purchase needed. Same bot as the URL-path mode `/botname`.

- **Resolution (botui, local — no API call):** `suite.rs` extracts the subdomain for any `*.{GB_PLATFORM_DOMAIN}` host; apex + reserved hosts (`chat.`, `www.`, `api.`, `docs.`, `store.`, …) serve the **default** bot.
- **Resolution (botserver API):** `GET /api/domains/resolve` matches exact `bot_domains` row → platform subdomain (`match_type: "platform_subdomain"`) → wildcard `*.domain` row.
- **Parameterized:** set `GB_PLATFORM_DOMAIN` (botserver **and** botui env) when self-hosting under a different platform domain. Default `generalbots.org`.
- **DNS:** wildcard record `*.{platform} → server IP` (CoreDNS zone) + Caddy site block per exposed host (or wildcard cert via DNS-01). New bot = **zero** infra change.

### Custom Domain Mode
Users with their own domain add a `bot_domains` row (Domain Manager UI) + point DNS at the server.

### How It Works
User visits a chat host (proxied to port 3000): botui reads `Host` header → platform subdomain resolved locally; otherwise calls `GET /api/domains/resolve?host=...` (anonymous — UI server resolves before any user auth) → botserver looks up `bot_domains` table → if found returns `{ found: true, bot_name, bot_id, org_id, branch_id }` → botui injects into `window.__INITIAL_BOT_NAME__` (same as URL path resolution) → fallback to URL path extraction.

### Database
**Table:** `bot_domains` (migration `6.5.18-bot-domains`)

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID PK | Unique identifier |
| `domain` | VARCHAR(255) UNIQUE | The hostname (e.g. `chat.pragmatismo.com.br`) |
| `bot_id` | UUID FK → bots | Which bot this domain routes to |
| `org_id` | UUID FK → organizations (optional) | Org scope for multi-tenant |
| `branch_id` | UUID FK → branches (optional) | Branch scope for multi-tenant |
| `created_at` / `updated_at` | TIMESTAMPTZ | Timestamps |

### API Endpoints (port 8080)
| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| `GET` | `/api/cloud/domains` | JWT + super admin | List all mappings |
| `POST` | `/api/cloud/domains` | JWT + super admin | Create mapping |
| `PUT` | `/api/cloud/domains/{id}` | JWT + super admin | Update mapping |
| `DELETE` | `/api/cloud/domains/{id}` | JWT + super admin | Delete mapping |
| `GET` | `/api/domains/resolve?host=` | Anonymous | Resolve hostname → bot name (exact → platform subdomain → wildcard) |

### Files Changed
| File | Change |
|------|--------|
| `botserver/migrations/6.5.18-bot-domains/up.sql` | Migration for `bot_domains` |
| `botserver/crates/botcloud/src/schema_ext.rs` | Diesel table definition |
| `botserver/crates/botcloud/src/domains.rs` | **New** — CRUD handlers + resolve |
| `botserver/crates/botcloud/src/lib.rs` | `pub mod domains` |
| `botserver/crates/botcloud/src/api.rs` | Routes + public access in JWT middleware |
| `botui/src/ui_server/suite.rs` | `resolve_bot_from_host()` + Host header check in `index()` |
| `botui/ui/cloud/domains.html` | **New** — Cloud UI page |

**Cloud UI page:** Domain Manager at `/domains` (port 4000, super-admin only, same gating as Vouchers). Create (domain + bot ID + optional org/branch), list, delete. It's the existing "Domains" nav link in the sidebar at `/store/apps` — now live.

**Add a mapping:**
```bash
curl -X POST http://localhost:8080/api/cloud/domains \
  -H "Authorization: Bearer <jwt-token>" -H "Content-Type: application/json" \
  -d '{"domain": "chat.generalbots.org", "bot_id": "<bot-uuid>", "org_id": null, "branch_id": null}'
```

**Test resolution:**
```bash
curl -s "http://localhost:8080/api/domains/resolve?host=chat.generalbots.org"
# mapped: {"found":true,"bot_id":"...","bot_name":"gbwebsite","org_id":null,"branch_id":null}
# unmapped: {"found":false}
```

**Architecture flow:**
```
Browser GET http://chat.generalbots.org/ (Host header) → Caddy/Proxy (80/443 → 3000)
→ botui suite.rs index(): 1. extract Host → 2. GET /api/domains/resolve → 3. bot_name="gbwebsite"
→ 4. inject window.__INITIAL_BOT_NAME__="gbwebsite" → 5. serve desktop.html → chat for gbwebsite
```

---

## Cloud Management Testing

### Ports
| Service | Port | Description |
|---------|------|-------------|
| Cloud UI (botui) | **4000** | Dashboard, store, offers, plans (**without** login/signup) |
| Cloud API (botserver) | **8080** | `/api/cloud/auth/signup`, `/api/cloud/auth/login`, etc. |
| Login UI (botui) | **5000** | `/login`, `/signup` — **only** service with auth |

### Plan Testing Flow (Free, Shared, Private Cloud)
Playwright via CDP to existing Chrome on 9222. Signup exclusively on port 5000, which redirects forms to the cloud API (8080):
```python
from playwright.async_api import async_playwright

async def test_cloud_plans():
    async with async_playwright() as p:
        browser = await p.chromium.connect_over_cdp("http://localhost:9222")
        ctx = browser.contexts[0]
        plans = [
            ('free', 'http://localhost:5000/signup'),
            ('shared', 'http://localhost:5000/signup?plan=shared'),
            ('private-cloud', 'http://localhost:5000/signup?plan=private-cloud'),
        ]
        for plan_id, url in plans:
            page = await ctx.new_page()
            await page.goto(url, wait_until='networkidle')
            await page.fill('#signup-name', 'Test User')
            await page.fill('#signup-botname', f'Bot{suffix}')
            await page.fill('#signup-email', f'{plan_id}-{suffix}@example.com')
            await page.fill('#signup-password', 'Test1234!')
            await page.click('#signup-btn')
            # DO NOT close the page — keep it open for inspection
```

### Expected behavior per plan
| Plan | Redirect | Created subscription |
|------|----------|----------------------|
| **free** | `/cloud/dashboard` | `billing_recurring` status=`active`, amount=`0.0` |
| **shared** | `/cloud/dashboard` | `billing_recurring` status=`trialing`, trial=14 days |
| **private-cloud** | `/cloud/store` | None (Custom/upon request) |

### Known bug (fixed)
**Symptom:** Signup with `free`/`shared` → error `"Insert ... subscription: insert or update on table 'billing_recurring' violates foreign key constraint 'billing_recurring_org_id_fkey'"`.
**Cause:** Migration `9.16-branch-id-isolation` changed the FK from `billing_recurring.org_id` to reference `branches(id)`, but `handle_signup` in `botserver/crates/botcloud/src/api.rs` still passed `org_id` instead of `branch_id`.
**Fix:** Replace `org_id` with `branch_id` in `create_free_subscription` and `create_trial_subscription` calls in `handle_signup`.

---

## Adding New Features Workflow

### Step 1: Plan
**Understand:** What problem does this solve? Which module owns it? What data structures? Security implications?
**Design checklist:** fits existing patterns? migrations? new API endpoints? affects existing features? error cases?

### Step 2: Implement (pattern)
1. **Types** in `botlib/src/models.rs` if shared across crates (`#[derive(Debug, Clone, Serialize, Deserialize)]`)
2. **Migration** `botserver/migrations/YYYY-MM-DD-HHMMSS_feature_name/up.sql` (`id UUID PRIMARY KEY DEFAULT gen_random_uuid()`, `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`)
3. **Diesel model** in `botserver/src/core/shared/models/core.rs` (`#[derive(Queryable, Insertable)] #[diesel(table_name = ...)]`)
4. **Business logic** `pub async fn create_feature(state: &AppState, ...) -> Result<NewFeature, Error>`
5. **API endpoint** in `botserver/src/api/routes.rs` (handler with `Extension<Arc<AppState>>` + `Json` payload)

**Security checklist:** input validation (`sanitize_identifier`) · auth required? · authorization checks? · rate limiting? · errors sanitized (`log_and_sanitize`)? · no `unwrap()`/`expect()`?

### Step 3: Add BASIC Keyword (if applicable)
`botserver/src/basic/keywords/new_feature.rs` — register via `engine.register_custom_syntax([...], true, move |context, inputs| ...)`. Bridge async → sync context with an `mpsc` channel + `std::thread::spawn` + `tokio::runtime::Builder::new_current_thread().enable_all()` + `rt.block_on(...)`, then `match result { Ok(f) => Ok(Dynamic::from(f.name)), Err(e) => Err(...) }`. Register with `if let Err(e) = ... { log::error!(...) }` — no `.expect()`.

### Step 4: Test
```bash
diesel migration run
./restart.sh
curl -X POST http://localhost:8080/api/features -H "Content-Type: application/json" -d '{"name": "test"}'
# create test.bas in /opt/gbo/data/testbot.gbai/testbot.gbdialog/ with: NEW_FEATURE "test"
tail -f botserver.log | grep -i "new_feature"
```
Integration test in `bottest/tests/new_feature_test.rs` (`#[tokio::test]`).

### Step 5: Expose the feature to the LLM (UI + API automation)
Every new/changed feature must be reachable by the LLM through **one of the two unified surfaces** — there is NO other way for chat/WhatsApp to act:

| Surface | Mechanism | Where it is registered |
|---------|-----------|------------------------|
| **UI/web** (desktop app) | `__ui_plan__` plans actions over apps; frontend calls REST endpoints directly | `ui_automation_instructions()` (ui_plan.rs) reads the app registry `all_apps()` (src/apps/registry.rs) — add/update the `AppDefinition` there with `id`, `title`, `description` and list the apps it can drive |
| **Chat/WhatsApp** (WS) | LLM uses BASIC keywords (`SEND MAIL`, `CREATE FILE`, …), `CALL "tool"` scripts, MCP tools via `.mcp.json`; raw REST is NOT reachable from chat | prompt files `PROMPT.md` / `PROMPT-{CHANNEL}.md` (load_system_prompt_for_channel in src/core/bot/ws/message.rs), `.mcp.json` in the bot's `.gbdialog/`, keyword registration |

Requirements when adding/changing an endpoint or feature:
1. **Register in the command catalog** so the LLM knows the call surface without a giant system prompt — add the endpoint to the compact catalog (api command catalog) with method, path, params, and the JSON it returns; never hardcode full schemas in prompts.
2. **UI features** must appear in `ui_automation_instructions()` (web channel only) so `__ui_plan__` can orchestrate them.
3. **Seed demo data** via `botsampledata` so the feature works end-to-end on a fresh drive (e.g. CSVs for cashflow imports, `billing_tax_rates` rows, sample product with `tax_rate`).
4. **Structured, summarized responses**: machine-readable JSON over prose — the LLM turns the JSON into user-facing text/visuals (e.g. diagnosis must return summary + detail, NOT generated markdown files).
5. If the feature is reachable only through an HTTP endpoint AND needs to be answerable in chat, register it in the **api command catalog** (declarative entry executed backend-side, results fed back to the LLM) — do not add a new BASIC keyword unless the operation is a chat-native primitive.
6. **RBAC-aware exposure:** mark admin-only capabilities (`admin_only: true` on the command, or a row in `rbac_api_permissions` for endpoint prefixes). The catalog filters admin-only entries for non-admin users via `resolve_user_role` — the injected manifest shortens per role, and the executor enforces the same rule server-side. Never rely on prompt text alone for authorization.

### Step 6: Document
Add to `botbook/src/features/` if user-facing; module README if developer-facing; inline comments; update API docs.

### Step 7: Commit & Deploy
```bash
git add .
git commit -m "feat: Add NEW_FEATURE keyword

- Adds new_features table with migrations
- Implements create_feature business logic
- Adds NEW_FEATURE BASIC keyword
- Includes API endpoint at POST /api/features
- Tests: Unit tests + integration test"
git push alm main
git push origin main
```

---

## Testing Strategy

- **Unit tests:** per-crate `tests/` or inline `#[cfg(test)]`; naming `test_` prefix; run `cargo test -p <crate>`
- **Binary-crate unit tests:** `botserver` has no library target, so `cargo test -p botserver` fails with *"no library targets found"* and tests nothing — its unit tests only build with `cargo test -p botserver --bin botserver` (the command CI and local runs must use)
- **Integration tests:** `bottest/` crate — full workflows across crates; run `cargo test -p bottest`
- **Tenant isolation regression suite:** `bottest/tests/integration/tenant_isolation.rs` — MUST stay green; guards drive/CRM/product tenant scoping (#1387, #1388, #1389). Run `cargo test -p bottest --features integration tenant_isolation` before touching any tenant/bucket/org-scoped handler.
- **Coverage goals:** critical paths 80%+ · ALL error paths tested · all security guards tested

### WhatsApp Integration Testing
1. Build with feature: `cargo check -p botserver --features whatsapp`
2. Bot secrets in Vault: `secret/gbo/00000000-0000-0000-0000-000000000000/{bot_id}/whatsapp` (whatsapp-api-key, whatsapp-verify-token, whatsapp-phone-number-id, whatsapp-business-account-id)
3. Use localtunnel (lt) as reverse proxy; check message storage:
   `psql -h localhost -U postgres -d botserver -c "SELECT * FROM messages WHERE bot_id = '<bot_id>' ORDER BY created_at DESC LIMIT 5;"`

---

## Debugging Rules

### CRITICAL ERROR HANDLING RULE
**STOP EVERYTHING WHEN ERRORS APPEAR.** When ANY error appears in logs during startup or operation:
1. **IMMEDIATELY STOP** — don't continue other tasks → 2. **IDENTIFY** the error + context → 3. **FIX** the root cause, not symptoms → 4. **VERIFY** resolution → 5. **ONLY THEN CONTINUE**. Never ignore or work around errors. **NEVER restart servers to "fix" errors — FIX THE ACTUAL PROBLEM.**

### Log Locations
| Component | Log File | What's Logged |
|-----------|----------|---------------|
| **botserver** | `botserver.log` | API requests, errors, script execution, **client navigation events** |
| **botui** | `botui.log` | UI rendering, WebSocket connections |
| **drive_monitor** | botserver logs, `[drive_monitor]` prefix | File sync, compilation |
| **client errors** | botserver logs, `CLIENT:` prefix | JavaScript errors, navigation events |

### Bug Fixing Workflow
1. **Reproduce & diagnose:** `grep -E " E | W " botserver.log | tail -20`; trace data flow backwards (UI → API → DB → cache), check logs at each layer.
   - *Example "Suggestions not showing":* check frontend requests (`grep "GET /api/suggestions"`), cache keys (`valkey-cli --scan --pattern "suggestions:*"`), generation (`grep "ADD_SUGGESTION"`), key format (`grep "Adding suggestion to Redis key"`).
2. **Find the code:** `grep -r "<keyword>" --include="*.rs"`; check `mod.rs` structure and related functions.
3. **Fix minimal changes:** wrong variable? (e.g. `user_id` vs `bot_id`) missing validation? race condition? config issue?
   ```rust
   // ❌ BAD: rewrite whole function  ✅ GOOD: fix only the bug
   - let key = format!("suggestions:{}:{}", user_session.user_id, session_id);
   + let key = format!("suggestions:{}:{}", user_session.bot_id, session_id);
   ```
   Search for similar bugs: `grep -n "user_session.user_id" botserver/src/basic/keywords/add_suggestion.rs`
4. **Test locally:** `cargo check -p botserver` → `./restart.sh` → trigger bug scenario in browser → check logs.
5. **Commit & deploy:**
   ```bash
   cd botserver && git add <file> && git commit -m "Fix: ..." 
   git push alm main && git push origin main
   cd .. && git add botserver && git commit -m "Update botserver: <desc>" && git push alm main && git push origin main
   ```
   ALM push triggers CI/CD; wait ~10 min for build + deploy; service auto-restarts; test in production after deployment.
6. **Document:** add to AGENTS-PROD.md if production-relevant (symptom, diagnosis, fix, prevention); update code comments.
   ```rust
   // Redis key format: suggestions:bot_id:session_id — must use bot_id (not user_id)
   ```

---

## Frontend & Performance Standards

### HTMX-First Approach
Use HTMX to minimize JavaScript; server returns HTML fragments, not JSON; use `hx-get`, `hx-post`, `hx-target`, `hx-swap`; WebSocket via htmx-ws extension.

### Local Assets Only — NO CDN
```html
<!-- ✅ CORRECT --> <script src="js/vendor/htmx.min.js"></script>
<!-- ❌ WRONG -->  <script src="https://unpkg.com/htmx.org@1.9.10"></script>
```

### Binary Size Optimization
Release profile: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`. Weekly: `cargo tree --duplicates`, `cargo machete` (remove unused deps), `cargo audit`. Use `default-features = false` and opt-in features.

### Linting & Code Quality
Clippy: MUST pass `cargo clippy --workspace` with **0 warnings**. No `#[allow(clippy::...)]` — FIX the code.

### Technical Debt Watch
Error-handling debt (`unwrap`/`expect` in production), performance debt (excessive `clone()`/`to_string()`), file size debt (>450 lines).

---

## Continuation Prompt (new sessions)

```
Continue on gb/ workspace. Follow AGENTS.md strictly:
1. Launch background checks: nohup cargo check -p <crate> > /tmp/<crate>_check.log 2>&1 &
2. While compiling: read files, analyze code, plan fixes
3. Poll results when convenient: tail /tmp/<crate>_check.log
4. Fix ALL warnings and errors - NO #[allow()] attributes
5. Delete unused code, don't suppress warnings
6. Remove unused parameters, don't prefix with _
7. Replace ALL unwrap()/expect() with proper error handling
8. Re-launch checks in background after fixes
9. Loop until 0 warnings, 0 errors
10. Refactor files >450 lines
```

**Memory & Main Directives:** LOOP AND COMPACT UNTIL 0 WARNINGS — MAXIMUM PRECISION. 0 warnings · 0 errors · trust project diagnostics · respect all rules · no `#[allow()]` · real code fixes only.
**Remember:** OFFLINE FIRST · BATCH BY FILE · WRITE ONCE · VERIFY LAST · NEVER BLOCK (nohup &) · ALWAYS PARALLEL · DELETE DEAD CODE · GIT WORKFLOW: ALWAYS push to ALL repositories (github, pragmatismo).

---

## Deploy in Prod Workflow

### CI/CD Pipeline (Primary Method)
1. **Push to ALM** (triggers CI/CD):
   ```bash
   cd botserver
   git push alm main
   git push origin main
   cd ..
   git add botserver
   git commit -m "Update botserver: <description>"
   git push alm main
   git push origin main
   ```
2. **Wait for CI programmatically** (ALM at port **4747**, NOT 3000; runner in container alm-ci):
   ```bash
   ALM_URL="http://<ALM_HOST>:4747"
   REPO="GeneralBots/BotServer"
   MAX_WAIT=600; ELAPSED=0
   while [ $ELAPSED -lt $MAX_WAIT ]; do
     STATUS=$(curl -sf "$ALM_URL/api/v1/repos/$REPO/actions/runs?per_page=1" | python3 -c "import sys,json; runs=json.load(sys.stdin); print(runs[0]['status'] if runs else 'unknown')")
     if [ "$STATUS" = "completed" ] || [ "$STATUS" = "failure" ] || [ "$STATUS" = "cancelled" ]; then
       echo "CI finished with status: $STATUS"; break
     fi
     echo "CI status: $STATUS (waiting ${ELAPSED}s...)"; sleep 15; ELAPSED=$((ELAPSED + 15))
   done
   # Alt 1: ssh <PROD_HOST> "sudo incus exec alm-ci -- tail -20 /opt/gbo/logs/forgejo-runner.log"
   # Alt 2: ssh <PROD_HOST> "sudo incus exec system -- stat -c '%y' /opt/gbo/bin/botserver"  (after sleep 240)
   ```
3. **Restart in prod** after binary updates:
   ```bash
   ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 <PROD_HOST> "sudo incus exec system -- pkill -f botserver || true"
   sleep 2
   ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 <PROD_HOST> "sudo incus exec system -- bash -c 'cd /opt/gbo/bin && RUST_LOG=info nohup ./botserver --noconsole > /opt/gbo/logs/stdout.log 2>&1 &'"
   ```
 4. **Verify deployment:** wait ~2 min for bootstrap → `curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/health` (via `sudo incus exec system`) → `tail -30 /opt/gbo/logs/stdout.log`.

### Manual Deploy (CI offline — see `~/.prod` for credentials)

When the alm-ci runner is offline, deploy manually (documented in `~/.prod` — read it from home, never commit it):

```bash
PASS=$(grep "^SRV1_PASS=" ~/.prod | cut -d= -f2-)
# 1. Transfer binary via gzip pipe (fast, no scp of 228MB raw)
gzip -c target/debug/botserver | sshpass -p "$PASS" ssh root@<SRV1_HOST> 'gunzip > /opt/gbo/bin/botserver.new'
# 2. STOP the service BEFORE replacing the binary ("text file busy" otherwise)
sshpass -p "$PASS" ssh root@<SRV1_HOST> "sudo incus exec bot -- systemctl stop botserver && \
  sudo incus file push /opt/gbo/bin/botserver.new bot/opt/gbo/bin/botserver && \
  sudo incus exec bot -- bash -c 'chmod +x /opt/gbo/bin/botserver && systemctl start botserver'"
# 3. Verify: sudo incus exec bot -- curl -s http://localhost:5858/health
```

**Gotchas learned (2026-08 session):**
- **Stop before push** — `incus file push` over a running binary fails with `text file busy`.
- **Vault sealed blocks bootstrap** — botserver retries forever ("Database pool creation failed … connection refused"); the old binary keeps working because credentials are cached in memory. Unseal: pull keys from the vault container `init.json`, run 3× `vault operator unseal <key>` with `VAULT_SKIP_VERIFY=true` (progress must reach 3/3; do it in one SSH session to keep the nonce, and DON'T chain the unseal with a failing SQL statement — a transaction rollback discards the group insert).
- **BotUI static files** — prod serves from `/opt/gbo/bin/ui/` (botui WorkingDirectory); push files there, NO botui restart needed (ServeDir reads per request).
- **JWT secret stability** — `saas_jwt_secret` is persisted into `conf/system/directory_config.json` (`resolve_saas_jwt_secret`); once set, cloud tokens survive botserver restarts. If the file lacks it, the old code generated a random UUID per boot → all tokens invalid after every restart.
- **Cloud login MUST verify passwords** — `handle_login` issues a token ONLY after Zitadel v2 sessions check (or the dev `admin-credentials.json` fallback with matching password). Never fall back to an unverified `crm_contacts` row. JWT `sub` = the Zitadel user id (resolve via `GET /v2/sessions/{sessionId}` → `session.factors.user.id` — NOT `userId`).
- **Chat admin sessions** — `/api/auth` accepts the cloud JWT (HMAC verified with the SaaS secret); the chat page captures `?token=` from the login redirect into `localStorage` (`chat-init.js`) and sends it. RBAC: `resolve_user_role` checks `rbac_user_groups` → group name containing "admin"; the user must exist in `users` (FK) with id = `UUIDv5("zitadel:{zitadel_user_id}")`.
- **Admin-only suggestions/tools** — gate in `start.bas` with `IF role = "admin" THEN … USE TOOL … / ADD SUGGESTION … END IF` (the `ROLE`/`role` scope variable comes from `resolve_user_role`); `tool_exec` only runs tools associated with the session.
- **prod DB cleanup** — Drive-discovered orgs get `cloud_workspaces` rows auto-created by `drive_monitors` (migration 6.5.31 adds a unique index on `branch_id`); older prod had a NON-unique index → `ON CONFLICT (branch_id)` fails → `ensure_cloud_workspace` falls back to select-then-upsert.
- **Always run the botserver via systemctl** inside the container (`sudo incus exec bot -- systemctl restart botserver`); health at `http://localhost:5858/health`.

### Production Container Architecture
| Container | Service | Port | Notes |
|-----------|---------|------|-------|
| bot / system | BotServer + Valkey | 5858/8080 | Main API + cache |
| vault | Vault | 8200 | Secrets (isolated) |
| tables | PostgreSQL | 5432 | Database |
| cache | Valkey | 6379 | Cache |
| drive | MinIO | 9000/9100 | Object storage |
| directory | Zitadel | 9000 | Identity provider |
| meet | LiveKit | 7880 | Video conferencing |
| vectordb | Qdrant | 6333 | Vector database |
| llm | llama.cpp | 8081 | Local LLM |
| email | Stalwart | 25/587 | Mail server |
| alm | Forgejo | 4747 | Git server (NOT 3000!) |
| alm-ci | Forgejo Runner | - | CI runner |
| proxy | Caddy | 80/443 | Reverse proxy |
| dns | CoreDNS | 53 | DNS resolution |
| webmail | Roundcube | behind proxy | PHP-FPM webmail |
| table-editor | NocoDB | behind proxy | Database UI |

**Important:** ALM (Forgejo) listens on port **4747**, not 3000. Runner token stored in `action_runner_token` table in `PROD-ALM` database.

### CI Runner Troubleshooting
| Symptom | Cause | Fix |
|---------|-------|-----|
| Runner not connecting | Wrong ALM port (3000 vs 4747) | Use port 4747 in runner registration |
| `registration file not found` | `.runner` missing/wrong format | Re-register: `forgejo-runner register --instance http://<ALM_HOST>:4747 --token <TOKEN> --name gbo --labels ubuntu-latest:docker://node:20-bookworm --no-interactive` |
| `unsupported protocol scheme` | `.runner` wrong JSON format | Delete `.runner` and re-register |
| `connection refused` to ALM | iptables blocking or ALM down | `sudo incus exec alm -- ss -tlnp \| grep 4747` |
| CI not picking up jobs | Runner not registered or labels mismatch | Check runner labels match workflow `runs-on` field |

### CI/CD (Forgejo Runner) details
- Config `/opt/gbo/bin/config.yaml` · init `/etc/systemd/system/alm-ci-runner.service` (runs as `gbuser`, NOT root)
- Logs `/opt/gbo/logs/out.log`, `/opt/gbo/logs/err.log` · auto-start via systemd
- Runner user `gbuser` (uid 1000) — all `/opt/gbo/` owned by `gbuser:gbuser`
- sccache at `/usr/local/bin/sccache` (`RUSTC_WRAPPER=sccache` in workflow) · workspace `/opt/gbo/data/` (NOT `/opt/gbo/ci/`)
- Cargo cache `/home/gbuser/.cargo/` · rustup `/home/gbuser/.rustup/` · SSH keys `/home/gbuser/.ssh/id_ed25519`
- Deploy: CI builds binary → tar+gzip via SSH → `/opt/gbo/bin/botserver` on bot container

---

## Production Operations Guide

### CRITICAL SAFETY RULES
1. **NEVER modify iptables rules without explicit confirmation** — always confirm exact rules, source IPs, ports, destinations before applying
2. **NEVER touch the PROD project without asking first** — no changes to production services/configs/containers without user approval
3. **ALWAYS backup files to `/tmp` before editing** — `cp /path/to/file /tmp/$(basename /path/to/file).bak-$(date +%Y%m%d%H%M%S)`

### Infrastructure Overview
Ubuntu LTS host · **Incus** (LXC) containers · Base `/opt/gbo/` · Data `/opt/gbo/data` · Bin `/opt/gbo/bin` · Conf `/opt/gbo/conf` · Logs `/opt/gbo/logs`

### Container Management
```bash
sudo incus list                                    # list all
sudo incus start|stop|restart <container>
sudo incus exec <container> -- bash                # exec into
sudo incus log <container> [--show-log]            # view logs
sudo incus file pull <container>/path /local/dest  # file ops
sudo incus file push /local/src <container>/path
sudo incus snapshot create <container> pre-change-$(date +%Y%m%d%H%M%S)   # snapshot before changes
```

### Service Management (inside container)
```bash
sudo incus exec <container> -- pgrep -a <process-name>          # check running
sudo incus exec <container> -- systemctl restart <service>      # restart
sudo incus exec <container> -- journalctl -u <service> -f       # follow logs
sudo incus exec <container> -- ss -tlnp                         # listening ports
```

### Quick Health Check
```bash
sudo incus list --format csv
for c in dns proxy tables system email webmail alm alm-ci drive table-editor; do
  echo -n "$c: "
  sudo incus exec $c -- pgrep -a $(case $c in
    dns) echo "coredns";; proxy) echo "caddy";; tables) echo "postgres";;
    system) echo "botserver";; email) echo "stalwart";; webmail) echo "php-fpm";;
    alm) echo "forgejo";; alm-ci) echo "runner";; drive) echo "minio";;
    table-editor) echo "nocodb";; esac) >/dev/null && echo OK || echo FAIL
done
```

### Network & NAT
External ports DNAT'd to container IPs via iptables; rules in `/etc/iptables.rules`.
**Critical pattern** — always use external interface to avoid loopback issues:
```
-A PREROUTING -i <external-iface> -p tcp --dport <port> -j DNAT --to-destination <container-ip>:<port>
```

**Typical port map:** 53 DNS · 80/443 HTTP(S) via Caddy · 5432 PostgreSQL (restricted) · 993 IMAPS · 465 SMTPS · 587 SMTP Submission · 25 SMTP (often blocked) · 4747 Forgejo (behind proxy) · 9000 MinIO API (internal) · 8200 Vault (isolated)

**Network diagnostics:**
```bash
sudo iptables -t nat -L -n | grep DNAT            # NAT rules
sudo incus exec <container> -- ping -c 3 8.8.8.8  # connectivity
sudo incus exec <container> -- dig <domain>       # DNS resolution
nc -zv <container-ip> <port>                      # port connectivity
```

### Key Service Operations
| Service | Config | Key facts |
|---------|--------|-----------|
| **DNS (CoreDNS)** | `/opt/gbo/conf/Corefile` | Zones `/opt/gbo/data/<domain>.zone`; test `dig @<dns-container-ip> <domain>` |
| **Database (PostgreSQL)** | `/opt/gbo/data` | Backup `pg_dump -U postgres -F c -f /tmp/backup.dump <dbname>`; Restore `pg_restore -U postgres -d <dbname> /tmp/backup.dump` |
| **Email (Stalwart)** | `/opt/gbo/conf/config.toml` | DKIM: TXT records `selector._domainkey.<domain>`; webmail behind proxy |
| **Proxy (Caddy)** | `/opt/gbo/conf/config` | Backup before edit; `caddy validate --config`; `caddy reload --config` |
| **Storage (MinIO/Drive)** | `/opt/gbo/data` | Console behind proxy; internal API `http://127.0.0.1:9100` (dev stack); buckets `{botname}.gbai`; creds in Vault `secret/gbo/drive` |
| **Bot System** | `/opt/gbo/bin/botserver` | BotServer + Valkey (port 6379) |

**Email recovery from crash:**
```bash
sudo incus exec email -- /opt/gbo/bin/stalwart -c /opt/gbo/conf/config.toml --help   # config validation
sudo incus exec email -- cat /opt/gbo/logs/stderr.log                                # error logs
# restore from snapshot if config corrupted:
sudo incus snapshot list email && sudo incus copy email/<snapshot> email-temp
sudo incus start email-temp
sudo incus file pull email-temp/opt/gbo/conf/config.toml /tmp/config.toml
sudo incus file push /tmp/config.toml email/opt/gbo/conf/config.toml
```

**Drive credentials from Vault (prod variant):**
```bash
export VAULT_ADDR=https://localhost:8200
export VAULT_CACERT=${WORKSPACE}/botserver-stack/conf/system/certificates/ca/ca.crt
export VAULT_TOKEN=$(cat /tmp/vault-token-gb 2>/dev/null || grep VAULT_TOKEN ${WORKSPACE}/botserver/.env | cut -d= -f2)
VAULT_BIN=${WORKSPACE}/botserver-stack/bin/vault/vault
DRIVE_ACCESSKEY=$($VAULT_BIN kv get -field=accesskey secret/gbo/drive)
DRIVE_SECRET=$($VAULT_BIN kv get -field=secret secret/gbo/drive)
DRIVE_PORT=$($VAULT_BIN kv get -field=port secret/gbo/drive)
```

**mc usage (install to /tmp, never repo root):**
```bash
curl -sL https://dl.min.io/client/mc/release/linux-amd64/mc -o /tmp/mc && chmod +x /tmp/mc
/tmp/mc alias set local http://127.0.0.1:${DRIVE_PORT} ${DRIVE_ACCESSKEY} ${DRIVE_SECRET} --api s3v4
/tmp/mc ls local/                                     # list bot buckets
/tmp/mc ls local/{botname}.gbai/{botname}.gbkb/docs/  # list KB files
/tmp/mc cp /path/to/file.xlsx local/{botname}.gbai/{botname}.gbkb/docs/   # upload KB
/tmp/mc mb local/testbot.gbai && /tmp/mc cp start.bas local/testbot.gbai/testbot.gbdialog/start.bas  # new bot
```

**Vault secret paths:** `secret/gbo/drive` (accesskey, secret, host, port) · `secret/gbo/tables` · `secret/gbo/cache` · `secret/gbo/directory` · `secret/gbo/llm` · `secret/gbo/vectordb` · `secret/gbo/alm` · `secret/gbo/email` · `secret/gbo/meet` · `secret/gbo/encryption`

### Backup & Recovery
```bash
sudo incus snapshot list <container>                        # list snapshots
sudo incus copy <container>/<snapshot> <container>-restored # restore
sudo incus start <container>-restored
sudo incus file pull <container>/<snapshot>/path/to/file .  # pull from snapshot w/o starting
```
Backup scripts: host config `/opt/gbo/bin/backup-local-host.sh` · remote S3 `/opt/gbo/bin/backup-remote.sh`

### Troubleshooting
```bash
# Container won't start
sudo incus list && sudo incus info <container>
sudo incus log <container> --show-log
sudo incus start <container> -v
# Service not running
sudo incus exec <container> -- pgrep -a <process>
sudo incus exec <container> -- ss -tlnp | grep <port>
sudo incus exec <container> -- tail -50 /opt/gbo/logs/stderr.log
# Email delivery issues
sudo incus exec email -- pgrep -a stalwart
nc -zv <email-ip> 993; nc -zv <email-ip> 465; nc -zv <email-ip> 587
dig TXT <selector>._domainkey.<domain>
sudo incus exec email -- tail -100 /opt/gbo/logs/email.log
```

### Maintenance
```bash
# Update container
sudo incus stop <container>
sudo incus snapshot create <container> pre-update-$(date +%Y%m%d)
sudo incus exec <container> -- apt update && apt upgrade -y
sudo incus start <container>
# Disk space
df -h /
sudo btrfs filesystem df /var/lib/incus
sudo incus exec <container> -- find /opt/gbo/logs -name "*.log.*" -mtime +7 -delete
```

### Container Tricks & Optimizations
```bash
# Resource limits
sudo incus config set <container> limits.cpu 2
sudo incus config set <container> limits.memory 4GiB
sudo incus config device set <container> root size 20GiB
# Profiles
sudo incus profile list
sudo incus profile add <container> <profile>
sudo incus copy <source> <target> --ephemeral
# Network optimization
sudo incus config device add <container> eth0 nic nictype=bridged parent=<bridge>
sudo incus config set <container> raw.lxc "lxc.net.0.ipv4.address=<ip>"
# Quick clone for testing
sudo incus snapshot create <container> test-base
sudo incus copy <container>/test-base <container>-test && sudo incus start <container>-test
# ... test safely ... then stop + delete
```

---

## Common Bug Fixes

### IF/THEN/ELSE Panic (`dag.rs`)
- **Symptom:** `IF/THEN/ELSE syntax: ParseError(BadInput(ImproperSymbol("$stmt$")))` during Rhai engine registration
- **Cause:** Rhai 1.25.x doesn't support `$stmt$` in `register_custom_syntax`; replaced by `$block$`
- **Fix in `botserver/crates/botbasic_core/src/keywords/dag.rs`:** `$stmt$` → `$block$` (3 occurrences: IF/THEN/ELSE, PARALLEL/AND, ON ERROR); `.expect("...")` → `if let Err(e) = ... { log::error!("...") }` (no panic in production); requires `rt-multi-thread` feature from tokio in `botbasic_core/Cargo.toml`

### Embedding URL Hardcoded to Empty
- **Symptom:** `Embedding server connection failed for : builder error` with empty URL
- **Cause:** `botqdrant/src/embedding.rs:14` hardcoded `let embedding_url = "".to_string()` instead of `self.llm_endpoint`
- **Fix:** `let embedding_url = &self.llm_endpoint;`

### Bot Access Denied (WebSocket)
- **Symptom:** `WS access denied for bot <name>: Access denied` or WebSocket closes with code 1006
- **Cause:** `bots.is_public = false` in the database
- **Fix:** `UPDATE bots SET is_public = true WHERE name = '<bot>';`

### Mail TLS Cert Expired (Stalwart/prod email container)
- **Symptom:** Clients get certificate-expired warnings on 993/465/587/995 for `mail.{domain}` (443/webmail via Caddy stay fine). Fixed on SRV1 2026-09-28: Stalwart 0.12.4 serves the cert **embedded inline** in `email:/opt/gbo/conf/config.toml` (`certificate.default.*`) and had no ACME section, so it never renewed; `%{file:...}%` references did NOT work on 0.12.4 (server served no cert). Caddy (proxy container) auto-renews the same hostname via TLS-ALPN on 443; its live storage is the **default** `/root/.local/share/caddy/certificates/` inside the proxy container (NOT `/opt/gbo/data/caddy`).
- **Fix (permanent):** `/usr/local/sbin/sync-stalwart-cert.sh` on the prod host runs daily (`/etc/cron.d/sync-stalwart-cert`, 04:17 UTC) — copies Caddy's renewed cert+key into the inline `certificate.default.*` blocks (key normalized SEC1→PKCS8), validates PEMs, backs up config with datetime suffix, deploys and restarts `email` **only when the cert changed** (log: `/var/log/sync-stalwart-cert.log`, rollback on failed restart). Manual run: `ssh <SRV1> '/usr/local/sbin/sync-stalwart-cert.sh'`.
- **Gotcha:** Stalwart has built-in fail2ban-style **in-memory throttling** — repeated `openssl s_client` probes (esp. hairpin NAT from the same public IP) get the source IP banned with `errno=104` / `no peer certificate available`. Bans decay on their own; verify TLS from another vantage point (e.g. the secondary host) before assuming the cert is broken. Backups: `email:/opt/gbo/conf/config.toml.bak-*` and `/tmp/stalwart-config-*.toml` on the host.

---

## BASIC Compile/Run Failure Playbook (media-filing saga, 2026-09)

The `classify_media` E2E surfaced five stacked bugs. Symptoms look identical
(`Expecting ';'`, tool aborts) but live in different layers — always pull the
runtime dumps before touching code:

### Runtime debug dumps are the FIRST step, not the last
`ScriptService::run` (botserver/src/basic/mod.rs:91) writes inside the bot
container: `/tmp/run_preprocessed.txt` (the transformed source — maps a Rhai
error line straight back to the offending statement), `/tmp/compile_error.txt`,
`/tmp/run_ast_input.txt`, `/tmp/run_result.txt`. Pull them via
`incus file pull bot:/tmp/run_preprocessed.txt` and read the exact line from the
error position before guessing.

### 1. Stale AST caches survive binary fixes
- **Symptom:** fixed a compiler transform, but the same Rhai error persists after deploy
- **Cause:** the work-dir `.ast` was compiled by the OLD binary; mangled statements
  are baked into the cached AST. Runtime self-heal (`ScriptService::run` re-applies
  only `convert_multiword_keywords`) cannot reverse an already-mangled statement.
- **Fix:** recompile with the new binary — commit the `.bas` in the bot's ALM repo (git sync →
  new ETag → drive_compiler recompiles) or hot-patch the work-dir `.ast` back to the keyword
  form so self-heal rewrites it. Never assume a deploy alone recompiles tools.

### 2. CREATE FILE rewrite read wrong regex groups
- **Symptom:** `let destination + ".meta.txt"create_file(...)` in run_preprocessed.txt
- **Cause:** `convert_multiword_keywords` read `get(2)/get(3)/get(4)` but the pattern
  has three groups (#1460)
- **Fix:** groups 1/2/3 = prefix/path/data; path may be a full expression.

### 3. Assignment heuristic let-prefixes calls containing '=' inside strings
- **Symptom:** `let create_file(p, "category=" + c)` — invalid Rhai
- **Cause:** `convert_if_then_syntax`'s `is_var_assignment` checked the RAW line;
  the `=` inside a string literal made a function call look like an assignment
- **Fix:** blank string literals first (`strip_string_literals`) before operator
  checks. Rule: any line-level heuristic must run on string-stripped code.

### 4. ON ERROR RESUME NEXT yielded UNIT, poisoning string pipelines
- **Symptom:** `Function not found: TRIM (())` after a trapped keyword failure (#1461)
- **Cause:** all 11 RESUME-NEXT error/timeout branches returned `Dynamic::UNIT` (Rhai
  `()`), so `x = DESCRIBE VIDEO path` assigned `()`; `LEN(TRIM(x))` then died. The
  script's `IF ERROR` handling was correct — the runtime value was not.
- **Fix:** RESUME-NEXT branches return an **empty string**, never UNIT. Contract:
  a trapped keyword yields `""` and sets the error flag.

### 5. Compiler regenerates an empty MCP manifest over the shipped one
- **Symptom:** `Variable not found: caption` when the LLM omits an optional arg (#1462)
- **Cause:** `compile_file()` always rewrites `{tool}.mcp.json` from the `.bas` (empty
  schema when the script has no PARAM headers), clobbering the rich manifest persisted
  by AutoTask shipped templates. `tool_exec` defaults omitted args from
  `input_schema.properties` — empty manifest → missing args become undeclared
  variables → abort. Note: RESUME NEXT does NOT trap undeclared-variable reads.
- **Fix:** drive_compiler syncs the Drive-side `{tool}.mcp.json` (source of truth)
  over the generated one after compiling; shipped templates persist the manifest
  BOTH at `tools/{tool}.mcp.json` and root `{tool}.mcp.json`.

### TODAY/NOW are functions, not variables (Rhai reality)
- **Symptom:** `Variable not found: TODAY`
- **Cause:** runtime registers `TODAY`/`NOW` as `register_fn` (call form); BASIC
  scripts write `TODAY` bare — Rhai reads a bare identifier as a variable.
- **Fix:** rewrite bare `TODAY`/`NOW` tokens to `TODAY()`/`NOW()` in the compile
  pipeline AND in the runtime self-heal so old ASTs heal on load.
- **⚠️ Rewrites must be IDEMPOTENT:** the self-heal runs on every load, including
  ASTs that already contain `TODAY()`. A lookahead-free pattern with an optional
  `\(\)` group and trailing `\b` backtracks (`TODAY()` → bare match) and produces
  the invalid `TODAY()()`. Normalize manually: after each match, consume an
  existing `()` pair when present, else append one. Test both forms.

### Channel adapters pass bot UUID where the pipeline expects a name
- **Symptom:** tool never runs on Telegram/WhatsApp; log:
  `get_session_tools error … Failed to get bot_id for bot 'f25671b3-…': Record not found`
- **Cause:** the Telegram webhook fills `UserMessage.bot_id` with the bot UUID, but
  `run_pipeline_for_channel` propagated it as the bot NAME; `get_session_tools`
  queried `bots.name = '<uuid>'` → zero tools for the session. Web chat passed the
  real name, hiding the bug.
- **Fix:** `channel_entry.rs` normalizes `bot_id` to the name (reverse UUID lookup)
  before resolving the UUID — one fix at the single channel entry point. When a
  channel feature works on web but not on a channel adapter, diff what each puts
  in `msg.bot_id` FIRST.

### E2E loop discipline for bot tools
Every fix → build → deploy → **force recompile** (commit the `.bas` in the bot's repo; git sync
recompiles) → re-test via
Chrome CDP browser upload (never WebSocket scripts) → verify Drive objects via
boto3 (`{bot}.gbdrive/media/...`). One error at a time; the dumps tell the layer.

---

## Reference: SaaS Product Listing Test Results (2026-06-28)

### Ports After Fix
| Port | Service | Cloud Access? | Suite Access? |
|------|---------|---------------|---------------|
| **3000** | Suite (botui) | ❌ 404 | ✅ Yes |
| **4000** | Cloud (botui) | ✅ Yes | ❌ N/A |
| **5000** | Login (botui) | ✅ Login | ✅ Login |
| **8080** | API (botserver) | ✅ API | ✅ API proxy |

**Fix:** `botui/src/ui_server/suite.rs` — added `/cloud/` blocking in the `index` handler (port 3000 fallback). Cloud pages now return 404 on port 3000.

### Populated Products (17 via automatic seed)
| Category | Items | SKUs |
|----------|-------|------|
| `plan` | 3 | free ($0), shared ($3.99), private-cloud (custom) |
| `infrastructure` | 8 | vps-small/medium/large, gpu-basic/advanced, storage-50/200/1000 |
| `communication` | 4 | number-local, number-tollfree, domain-com, domain-org |
| `llm-tokens` | 2 | llm-tokens-1m ($9.99), llm-tokens-10m ($79.99) |

### API Endpoints Verified
| Endpoint | Format | Status |
|----------|--------|--------|
| `GET /api/products/items` | JSON (17 items) | ✅ 200 |
| `GET /api/catalog/prices.json` | JSON-LD Schema.org (17 items) | ✅ 200 |
| `GET /api/products/categories` | JSON (7 categories) | ✅ 200 |

### Cloud Pages Verified (port 4000)
| Page | URL | Screenshot |
|------|-----|-----------|
| Plans | `/cloud/plans` | `/tmp/saas_admin_plans.png` |
| Store | `/cloud/store` | `/tmp/saas_admin_store.png` |
| Signup | `/cloud/signup` | `/tmp/saas_admin_signup.png` |
| Dashboard | `/cloud/dashboard` | `/tmp/saas_admin_dashboard.png` |
| Offers | `/cloud/offers` | `/tmp/saas_admin_offers.png` |

**Evidence slideshow:** `python3 -m http.server 9090 --directory /tmp` → open `http://localhost:9090/slideshow.html`
