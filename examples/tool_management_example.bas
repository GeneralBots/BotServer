REM Tool Management Example
REM This script demonstrates how to manage multiple tools in a conversation
REM using ADD_TOOL, REMOVE_TOOL, CLEAR_TOOLS, and LIST_TOOLS keywords

REM Step 1: List current tools (should be empty at start)
PRINT "=== Initial Tool Status ==="
LIST_TOOLS

REM Step 2: Add multiple tools to the conversation
PRINT ""
PRINT "=== Adding Tools ==="
ADD_TOOL ".gbdialog/enrollment.bas"
ADD_TOOL ".gbdialog/payment.bas"
ADD_TOOL ".gbdialog/support.bas"

REM Step 3: List all active tools
PRINT ""
PRINT "=== Current Active Tools ==="
LIST_TOOLS

REM Step 4: The LLM can now use all these tools in the conversation
PRINT ""
PRINT "All tools are now available for the AI assistant to use!"
PRINT "The assistant can call any of these tools based on user queries."

REM Step 5: Remove a specific tool
PRINT ""
PRINT "=== Removing Support Tool ==="
REMOVE_TOOL ".gbdialog/support.bas"

REM Step 6: List tools again to confirm removal
PRINT ""
PRINT "=== Tools After Removal ==="
LIST_TOOLS

REM Step 7: Add another tool
PRINT ""
PRINT "=== Adding Analytics Tool ==="
ADD_TOOL ".gbdialog/analytics.bas"

REM Step 8: Show final tool list
PRINT ""
PRINT "=== Final Tool List ==="
LIST_TOOLS

REM Step 9: Clear all tools (optional - uncomment to use)
REM PRINT ""
REM PRINT "=== Clearing All Tools ==="
REM CLEAR_TOOLS
REM LIST_TOOLS

PRINT ""
PRINT "=== Tool Management Complete ==="
PRINT "Tools can be dynamically added/removed during conversation"
PRINT "Each tool remains active only for this session"
