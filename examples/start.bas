REM ============================================================================
REM General Bots - Main Start Script
REM ============================================================================
REM This is the main entry point script that:
REM 1. Registers tools as MCP endpoints
REM 2. Activates general knowledge bases
REM 3. Configures the bot's behavior and capabilities
REM 4. Sets up the initial context
REM ============================================================================

REM ============================================================================
REM Bot Configuration
REM ============================================================================

PRINT "=========================================="
PRINT "General Bots - Starting up..."
PRINT "=========================================="

REM Set bot information
SET BOT MEMORY "bot_name", "General Assistant"
SET BOT MEMORY "bot_version", "2.0.0"
SET BOT MEMORY "startup_time", NOW()

REM ============================================================================
REM Register Business Tools as MCP Endpoints
REM ============================================================================
REM These tools become available as HTTP endpoints and can be called
REM by external systems or other bots through the Model Context Protocol
REM ============================================================================

PRINT "Registering business tools..."

REM Enrollment tool - handles user registration
REM Creates endpoint: POST /default/enrollment
ADD_TOOL "enrollment.bas" as MCP
PRINT "  ✓ Enrollment tool registered"

REM Pricing tool - provides product information and prices
REM Creates endpoint: POST /default/pricing
ADD_TOOL "pricing.bas" as MCP
PRINT "  ✓ Pricing tool registered"

REM Customer support tool - handles support inquiries
REM ADD_TOOL "support.bas" as MCP
REM PRINT "  ✓ Support tool registered"

REM Order processing tool
REM ADD_TOOL "order_processing.bas" as MCP
REM PRINT "  ✓ Order processing tool registered"

REM ============================================================================
REM Activate General Knowledge Bases
REM ============================================================================
REM These KBs are always available and provide general information
REM Documents in these folders are automatically indexed and searchable
REM ============================================================================

PRINT "Activating knowledge bases..."

REM General company documentation
REM Contains: company policies, procedures, guidelines
ADD_KB "generalmdsandpdfs"
PRINT "  ✓ General documentation KB activated"

REM Product catalog and specifications
REM Contains: product brochures, technical specs, comparison charts
ADD_KB "productbrochurespdfsanddocs"
PRINT "  ✓ Product catalog KB activated"

REM FAQ and help documentation
REM Contains: frequently asked questions, troubleshooting guides
ADD_KB "faq_and_help"
PRINT "  ✓ FAQ and Help KB activated"

REM Training materials
REM Contains: training videos transcripts, tutorials, how-to guides
REM ADD_KB "training_materials"
REM PRINT "  ✓ Training materials KB activated"

REM ============================================================================
REM Add External Documentation Sources
REM ============================================================================
REM These websites are crawled and indexed for additional context
REM Useful for keeping up-to-date with external documentation
REM ============================================================================

PRINT "Indexing external documentation..."

REM Company public documentation
REM ADD_WEBSITE "https://docs.generalbots.ai/"
REM PRINT "  ✓ General Bots documentation indexed"

REM Product knowledge base
REM ADD_WEBSITE "https://example.com/knowledge-base"
REM PRINT "  ✓ Product knowledge base indexed"

REM ============================================================================
REM Set Default Answer Mode
REM ============================================================================
REM Answer Modes:
REM   0 = Direct         - Simple LLM responses
REM   1 = WithTools      - LLM with tool calling capability
REM   2 = DocumentsOnly  - Search KB only, no LLM generation
REM   3 = WebSearch      - Include web search in responses
REM   4 = Mixed          - Intelligent mix of KB + Tools (RECOMMENDED)
REM ============================================================================

SET CONTEXT "answer_mode", "4"
PRINT "Answer mode set to: Mixed (KB + Tools)"

REM ============================================================================
REM Set Welcome Message
REM ============================================================================

welcome_message = "👋 Hello! I'm your General Assistant.\n\n"
welcome_message = welcome_message + "I can help you with:\n"
welcome_message = welcome_message + "• **Enrollment** - Register new users and manage accounts\n"
welcome_message = welcome_message + "• **Product Information** - Get prices, specifications, and availability\n"
welcome_message = welcome_message + "• **Documentation** - Access our complete knowledge base\n"
welcome_message = welcome_message + "• **General Questions** - Ask me anything about our services\n\n"
welcome_message = welcome_message + "I have access to multiple knowledge bases and can search through:\n"
welcome_message = welcome_message + "📚 Company policies and procedures\n"
welcome_message = welcome_message + "📦 Product catalogs and technical specifications\n"
welcome_message = welcome_message + "❓ FAQs and troubleshooting guides\n\n"
welcome_message = welcome_message + "How can I assist you today?"

SET BOT MEMORY "welcome_message", welcome_message

REM ============================================================================
REM Set Conversation Context
REM ============================================================================

SET CONTEXT "active_tools", "enrollment,pricing"
SET CONTEXT "available_kbs", "generalmdsandpdfs,productbrochurespdfsanddocs,faq_and_help"
SET CONTEXT "capabilities", "enrollment,pricing,documentation,support"

REM ============================================================================
REM Configure Behavior Parameters
REM ============================================================================

REM Response style
SET CONTEXT "response_style", "professional_friendly"
SET CONTEXT "language", "en"
SET CONTEXT "max_context_documents", "5"

REM Knowledge retrieval settings
SET CONTEXT "kb_similarity_threshold", "0.7"
SET CONTEXT "kb_max_results", "3"

REM Tool calling settings
SET CONTEXT "tool_timeout_seconds", "30"
SET CONTEXT "auto_call_tools", "true"

REM ============================================================================
REM Initialize Analytics
REM ============================================================================

session_id = SESSION_ID()
bot_id = BOT_ID()

SAVE "bot_sessions.csv", session_id, bot_id, NOW(), "initialized"

PRINT "Session initialized: " + session_id

REM ============================================================================
REM Set Up Event Handlers
REM ============================================================================
REM These handlers respond to specific events or keywords
REM ============================================================================

REM ON "help" DO
REM     TALK welcome_message
REM END ON

REM ON "reset" DO
REM     CLEAR CONTEXT
REM     TALK "Context cleared. How can I help you?"
REM END ON

REM ON "capabilities" DO
REM     caps = "I can help with:\n"
REM     caps = caps + "• User enrollment and registration\n"
REM     caps = caps + "• Product pricing and information\n"
REM     caps = caps + "• Documentation search\n"
REM     caps = caps + "• General support questions\n"
REM     TALK caps
REM END ON

REM ============================================================================
REM Schedule Periodic Tasks
REM ============================================================================
REM These tasks run automatically at specified intervals
REM ============================================================================

REM Update KB indices every 6 hours
REM SET SCHEDULE "0 */6 * * *" DO
REM     PRINT "Refreshing knowledge base indices..."
REM     REM Knowledge bases are automatically refreshed by KB Manager
REM END SCHEDULE

REM Generate daily analytics report
REM SET SCHEDULE "0 0 * * *" DO
REM     PRINT "Generating daily analytics..."
REM     REM Generate report logic here
REM END SCHEDULE

REM ============================================================================
REM Startup Complete
REM ============================================================================

PRINT "=========================================="
PRINT "✓ Startup complete!"
PRINT "✓ Tools registered: enrollment, pricing"
PRINT "✓ Knowledge bases active: 3"
PRINT "✓ Answer mode: Mixed (4)"
PRINT "✓ Session ID: " + session_id
PRINT "=========================================="

REM Display welcome message to user
TALK welcome_message

REM ============================================================================
REM Ready to serve!
REM ============================================================================
