# Tool Management System

## Overview

The Bot Server now supports **multiple tool associations** per user session. This allows users to dynamically load, manage, and use multiple BASIC tools during a single conversation without needing to restart or change sessions.

## Features

- **Multiple Tools per Session**: Associate multiple compiled BASIC tools with a single conversation
- **Dynamic Management**: Add or remove tools on-the-fly during a conversation
- **Session Isolation**: Each session has its own independent set of active tools
- **Persistent Associations**: Tool associations are stored in the database and survive across requests
- **Real Database Implementation**: No SQL placeholders - fully implemented with Diesel ORM

## Database Schema

### `session_tool_associations` Table

```sql
CREATE TABLE IF NOT EXISTS session_tool_associations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    added_at TEXT NOT NULL,
    UNIQUE(session_id, tool_name)
);
```

**Indexes:**
- `idx_session_tool_session` on `session_id`
- `idx_session_tool_name` on `tool_name`

The UNIQUE constraint ensures a tool cannot be added twice to the same session.

## BASIC Keywords

### `ADD_TOOL`

Adds a compiled tool to the current session, making it available for the LLM to call.

**Syntax:**
```basic
ADD_TOOL "<path_to_tool>"
```

**Example:**
```basic
ADD_TOOL ".gbdialog/enrollment.bas"
ADD_TOOL ".gbdialog/payment.bas"
ADD_TOOL ".gbdialog/support.bas"
```

**Behavior:**
- Validates that the tool exists in the `basic_tools` table
- Verifies the tool is active (`is_active = 1`)
- Checks the tool belongs to the current bot
- Inserts into `session_tool_associations` table
- Returns success message or error if tool doesn't exist
- If tool is already associated, reports it's already active

**Returns:**
- Success: `"Tool 'enrollment' is now available in this conversation"`
- Already added: `"Tool 'enrollment' is already available in this conversation"`
- Error: `"Tool 'enrollment' is not available. Make sure the tool file is compiled and active."`

---

### `REMOVE_TOOL`

Removes a tool association from the current session.

**Syntax:**
```basic
REMOVE_TOOL "<path_to_tool>"
```

**Example:**
```basic
REMOVE_TOOL ".gbdialog/support.bas"
```

**Behavior:**
- Removes the tool from `session_tool_associations` for this session
- Does not delete the compiled tool itself
- Only affects the current session

**Returns:**
- Success: `"Tool 'support' has been removed from this conversation"`
- Not found: `"Tool 'support' was not active in this conversation"`

---

### `CLEAR_TOOLS`

Removes all tool associations from the current session.

**Syntax:**
```basic
CLEAR_TOOLS
```

**Example:**
```basic
CLEAR_TOOLS
```

**Behavior:**
- Removes all entries in `session_tool_associations` for this session
- Does not affect other sessions
- Does not delete compiled tools

**Returns:**
- Success: `"All 3 tool(s) have been removed from this conversation"`
- No tools: `"No tools were active in this conversation"`

---

### `LIST_TOOLS`

Lists all tools currently associated with the session.

**Syntax:**
```basic
LIST_TOOLS
```

**Example:**
```basic
LIST_TOOLS
```

**Output:**
```
Active tools in this conversation (3):
1. enrollment
2. payment
3. analytics
```

**Returns:**
- With tools: Lists all active tools with numbering
- No tools: `"No tools are currently active in this conversation"`

---

## How It Works

### Tool Loading Flow

1. **User calls `ADD_TOOL` in BASIC script**
   ```basic
   ADD_TOOL ".gbdialog/enrollment.bas"
   ```

2. **System validates tool exists**
   - Queries `basic_tools` table
   - Checks `bot_id` matches current bot
   - Verifies `is_active = 1`

3. **Association is created**
   - Inserts into `session_tool_associations`
   - Uses UNIQUE constraint to prevent duplicates
   - Stores session_id, tool_name, and timestamp

4. **LLM requests include tools**
   - When processing prompts, system loads all tools from `session_tool_associations`
   - Tools are added to the LLM's available function list
   - LLM can now call any associated tool

### Integration with Prompt Processor

The `PromptProcessor::get_available_tools()` method now:

1. Loads tool stack from bot configuration (existing behavior)
2. **NEW**: Queries `session_tool_associations` for the current session
3. Adds all associated tools to the available tools list
4. Maintains backward compatibility with legacy `current_tool` field

**Code Example:**
```rust
// From src/context/prompt_processor.rs
if let Ok(mut conn) = self.state.conn.lock() {
    match get_session_tools(&mut *conn, &session.id) {
        Ok(session_tools) => {
            for tool_name in session_tools {
                if !tools.iter().any(|t| t.tool_name == tool_name) {
                    tools.push(ToolContext {
                        tool_name: tool_name.clone(),
                        description: format!("Tool: {}", tool_name),
                        endpoint: format!("/default/{}", tool_name),
                    });
                }
            }
        }
        Err(e) => error!("Failed to load session tools: {}", e),
    }
}
```

---

## Rust API

### Public Functions

All functions are in `botserver/src/basic/keywords/add_tool.rs`:

```rust
/// Get all tools associated with a session
pub fn get_session_tools(
    conn: &mut PgConnection,
    session_id: &Uuid,
) -> Result<Vec<String>, diesel::result::Error>

/// Remove a tool association from a session
pub fn remove_session_tool(
    conn: &mut PgConnection,
    session_id: &Uuid,
    tool_name: &str,
) -> Result<usize, diesel::result::Error>

/// Clear all tool associations for a session
pub fn clear_session_tools(
    conn: &mut PgConnection,
    session_id: &Uuid,
) -> Result<usize, diesel::result::Error>
```

**Usage Example:**
```rust
use crate::basic::keywords::add_tool::get_session_tools;

let tools = get_session_tools(&mut conn, &session_id)?;
for tool_name in tools {
    println!("Active tool: {}", tool_name);
}
```

---

## Use Cases

### 1. Progressive Tool Loading

Start with basic tools and add more as needed:

```basic
REM Start with customer service tool
ADD_TOOL ".gbdialog/customer_service.bas"

REM If user needs technical support, add that tool
IF user_needs_technical_support THEN
    ADD_TOOL ".gbdialog/technical_support.bas"
END IF

REM If billing question, add payment tool
IF user_asks_about_billing THEN
    ADD_TOOL ".gbdialog/billing.bas"
END IF
```

### 2. Context-Aware Tool Management

Different tools for different conversation stages:

```basic
REM Initial greeting phase
ADD_TOOL ".gbdialog/greeting.bas"
HEAR "start"

REM Main interaction phase
REMOVE_TOOL ".gbdialog/greeting.bas"
ADD_TOOL ".gbdialog/enrollment.bas"
ADD_TOOL ".gbdialog/faq.bas"
HEAR "continue"

REM Closing phase
CLEAR_TOOLS
ADD_TOOL ".gbdialog/feedback.bas"
HEAR "finish"
```

### 3. Department-Specific Tools

Route to different tool sets based on department:

```basic
GET "/api/user/department" AS department

IF department = "sales" THEN
    ADD_TOOL ".gbdialog/lead_capture.bas"
    ADD_TOOL ".gbdialog/quote_generator.bas"
    ADD_TOOL ".gbdialog/crm_integration.bas"
ELSE IF department = "support" THEN
    ADD_TOOL ".gbdialog/ticket_system.bas"
    ADD_TOOL ".gbdialog/knowledge_base.bas"
    ADD_TOOL ".gbdialog/escalation.bas"
END IF
```

### 4. A/B Testing Tools

Test different tool combinations:

```basic
GET "/api/user/experiment_group" AS group

IF group = "A" THEN
    ADD_TOOL ".gbdialog/tool_variant_a.bas"
ELSE
    ADD_TOOL ".gbdialog/tool_variant_b.bas"
END IF

REM Both groups get common tools
ADD_TOOL ".gbdialog/common_tools.bas"
```

---

## Answer Modes

The system respects the session's `answer_mode`:

- **Mode 0 (Direct)**: No tools used
- **Mode 1 (WithTools)**: Uses associated tools + legacy `current_tool`
- **Mode 2 (DocumentsOnly)**: Only KB documents, no tools
- **Mode 3 (WebSearch)**: Web search enabled
- **Mode 4 (Mixed)**: Tools from `session_tool_associations` + KB documents

Set answer mode via session configuration or dynamically.

---

## Best Practices

### 1. **Validate Before Use**
Always check if a tool is successfully added:
```basic
ADD_TOOL ".gbdialog/payment.bas"
LIST_TOOLS  REM Verify it was added
```

### 2. **Clean Up When Done**
Remove tools that are no longer needed to improve LLM performance:
```basic
REMOVE_TOOL ".gbdialog/onboarding.bas"
```

### 3. **Use LIST_TOOLS for Debugging**
When developing, list tools to verify state:
```basic
LIST_TOOLS
PRINT "Current tools listed above"
```

### 4. **Tool Names are Simple**
Tool names are extracted from paths automatically:
- `.gbdialog/enrollment.bas` → `enrollment`
- `payment.bas` → `payment`

### 5. **Session Isolation**
Each session maintains its own tool list. Tools added in one session don't affect others.

### 6. **Compile Before Adding**
Ensure tools are compiled and present in the `basic_tools` table before attempting to add them. The DriveMonitor service handles compilation automatically when `.bas` files are saved.

---

## Migration Guide

### Upgrading from Single Tool (`current_tool`)

**Before (Legacy):**
```rust
// Single tool stored in session.current_tool
session.current_tool = Some("enrollment".to_string());
```

**After (Multi-Tool):**
```basic
ADD_TOOL ".gbdialog/enrollment.bas"
ADD_TOOL ".gbdialog/payment.bas"
ADD_TOOL ".gbdialog/support.bas"
```

**Backward Compatibility:**
The system still supports the legacy `current_tool` field. If set, it will be included in the available tools list alongside tools from `session_tool_associations`.

---

## Technical Implementation Details

### Database Operations

All operations use Diesel ORM with proper error handling:

```rust
// Insert with conflict resolution
diesel::insert_into(session_tool_associations::table)
    .values((/* ... */))
    .on_conflict((session_id, tool_name))
    .do_nothing()
    .execute(&mut *conn)

// Delete specific tool
diesel::delete(
    session_tool_associations::table
        .filter(session_id.eq(&session_id_str))
        .filter(tool_name.eq(tool_name))
).execute(&mut *conn)

// Load all tools
session_tool_associations::table
    .filter(session_id.eq(&session_id_str))
    .select(tool_name)
    .load::<String>(&mut *conn)
```

### Thread Safety

All operations use Arc<Mutex<PgConnection>> for thread-safe database access:

```rust
let mut conn = state.conn.lock().map_err(|e| {
    error!("Failed to acquire database lock: {}", e);
    format!("Database connection error: {}", e)
})?;
```

### Async Execution

Keywords spawn async tasks using Tokio runtime to avoid blocking the Rhai engine:

```rust
std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build();
    // ... execute async operation
});
```

---

## Error Handling

### Common Errors

1. **Tool Not Found**
   - Message: `"Tool 'xyz' is not available. Make sure the tool file is compiled and active."`
   - Cause: Tool doesn't exist in `basic_tools` or is inactive
   - Solution: Compile the tool or check bot_id matches

2. **Database Lock Error**
   - Message: `"Database connection error: ..."`
   - Cause: Failed to acquire database mutex
   - Solution: Check database connection health

3. **Timeout**
   - Message: `"ADD_TOOL timed out"`
   - Cause: Operation took longer than 10 seconds
   - Solution: Check database performance

### Error Recovery

All operations are atomic - if they fail, no partial state is committed:

```basic
ADD_TOOL ".gbdialog/nonexistent.bas"
REM Error returned, no changes made
LIST_TOOLS
REM Still shows previous tools only
```

---

## Performance Considerations

### Database Indexes

The following indexes ensure fast lookups:
- `idx_session_tool_session`: Fast retrieval of all tools for a session
- `idx_session_tool_name`: Fast tool name lookups
- UNIQUE constraint on (session_id, tool_name): Prevents duplicates

### Query Optimization

Tools are loaded once per prompt processing:
```rust
// Efficient batch load
let tools = get_session_tools(&mut conn, &session.id)?;
```

### Memory Usage

- Tool associations are lightweight (only stores IDs and names)
- No tool code is duplicated in the database
- Compiled tools are referenced, not copied

---

## Security

### Access Control

- Tools are validated against bot_id
- Users can only add tools belonging to their current bot
- Session isolation prevents cross-session access

### Input Validation

- Tool names are extracted and sanitized
- SQL injection prevented by Diesel parameterization
- Empty tool names are rejected

---

## Testing

### Example Test Script

See `botserver/examples/tool_management_example.bas` for a complete working example.

### Unit Testing

Test the Rust API directly:

```rust
#[test]
fn test_multiple_tool_association() {
    let mut conn = establish_connection();
    let session_id = Uuid::new_v4();
    
    // Add tools
    add_tool(&mut conn, &session_id, "tool1").unwrap();
    add_tool(&mut conn, &session_id, "tool2").unwrap();
    
    // Verify
    let tools = get_session_tools(&mut conn, &session_id).unwrap();
    assert_eq!(tools.len(), 2);
    
    // Remove one
    remove_session_tool(&mut conn, &session_id, "tool1").unwrap();
    let tools = get_session_tools(&mut conn, &session_id).unwrap();
    assert_eq!(tools.len(), 1);
    
    // Clear all
    clear_session_tools(&mut conn, &session_id).unwrap();
    let tools = get_session_tools(&mut conn, &session_id).unwrap();
    assert_eq!(tools.len(), 0);
}
```

---

## Future Enhancements

Potential improvements:

1. **Tool Priority/Ordering**: Specify which tools to try first
2. **Tool Groups**: Add/remove sets of related tools together
3. **Auto-Cleanup**: Remove tool associations when session ends
4. **Tool Statistics**: Track which tools are used most frequently
5. **Conditional Tool Loading**: Load tools based on LLM decisions
6. **Tool Permissions**: Fine-grained control over which users can use which tools

---

## Troubleshooting

### Tools Not Appearing

1. Check compilation:
   ```sql
   SELECT * FROM basic_tools WHERE tool_name = 'enrollment';
   ```

2. Verify bot_id matches:
   ```sql
   SELECT bot_id FROM basic_tools WHERE tool_name = 'enrollment';
   ```

3. Check is_active flag:
   ```sql
   SELECT is_active FROM basic_tools WHERE tool_name = 'enrollment';
   ```

### Tools Not Being Called

1. Verify answer_mode is 1 or 4
2. Check tool is in session associations:
   ```sql
   SELECT * FROM session_tool_associations WHERE session_id = '<your-session-id>';
   ```
3. Review LLM logs to see if tool was included in prompt

### Database Issues

Check connection:
```bash
psql -h localhost -U your_user -d your_database
\dt session_tool_associations
```

---

## References

- **Schema**: `botserver/migrations/6.0.3.sql`
- **Implementation**: `botserver/src/basic/keywords/add_tool.rs`
- **Prompt Integration**: `botserver/src/context/prompt_processor.rs`
- **Models**: `botserver/src/shared/models.rs`
- **Example**: `botserver/examples/tool_management_example.bas`

---

## License

This feature is part of the Bot Server project. See the main LICENSE file for details.