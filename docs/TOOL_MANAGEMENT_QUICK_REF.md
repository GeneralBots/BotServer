# Tool Management Quick Reference

## 🚀 Quick Start

### Add a Tool
```basic
ADD_TOOL ".gbdialog/enrollment.bas"
```

### Remove a Tool
```basic
REMOVE_TOOL ".gbdialog/enrollment.bas"
```

### List Active Tools
```basic
LIST_TOOLS
```

### Clear All Tools
```basic
CLEAR_TOOLS
```

---

## 📋 Common Patterns

### Multiple Tools in One Session
```basic
ADD_TOOL ".gbdialog/enrollment.bas"
ADD_TOOL ".gbdialog/payment.bas"
ADD_TOOL ".gbdialog/support.bas"
LIST_TOOLS
```

### Progressive Loading
```basic
REM Start with basic tool
ADD_TOOL ".gbdialog/greeting.bas"

REM Add more as needed
IF user_needs_help THEN
    ADD_TOOL ".gbdialog/support.bas"
END IF
```

### Tool Rotation
```basic
REM Switch tools for different phases
REMOVE_TOOL ".gbdialog/onboarding.bas"
ADD_TOOL ".gbdialog/main_menu.bas"
```

---

## ⚡ Key Features

- ✅ **Multiple tools per session** - No limit on number of tools
- ✅ **Dynamic management** - Add/remove during conversation
- ✅ **Session isolation** - Each session has independent tool list
- ✅ **Persistent** - Survives across requests
- ✅ **Real database** - Fully implemented with Diesel ORM

---

## 🔍 What Happens Behind the Scenes

1. **ADD_TOOL** → Validates tool exists → Inserts into `session_tool_associations` table
2. **Prompt Processing** → Loads all tools for session → LLM can call them
3. **REMOVE_TOOL** → Deletes association → Tool no longer available
4. **CLEAR_TOOLS** → Removes all associations for session

---

## 📊 Database Table

```sql
CREATE TABLE session_tool_associations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    added_at TEXT NOT NULL,
    UNIQUE(session_id, tool_name)
);
```

---

## 🎯 Use Cases

### Customer Service Bot
```basic
ADD_TOOL ".gbdialog/faq.bas"
ADD_TOOL ".gbdialog/ticket_system.bas"
ADD_TOOL ".gbdialog/escalation.bas"
```

### E-commerce Bot
```basic
ADD_TOOL ".gbdialog/product_search.bas"
ADD_TOOL ".gbdialog/cart_management.bas"
ADD_TOOL ".gbdialog/checkout.bas"
ADD_TOOL ".gbdialog/order_tracking.bas"
```

### HR Bot
```basic
ADD_TOOL ".gbdialog/leave_request.bas"
ADD_TOOL ".gbdialog/payroll_info.bas"
ADD_TOOL ".gbdialog/benefits.bas"
```

---

## ⚠️ Important Notes

- Tool must be compiled and in `basic_tools` table
- Tool must have `is_active = 1`
- Tool must belong to current bot (`bot_id` match)
- Path can be with or without `.gbdialog/` prefix
- Tool names auto-extracted: `enrollment.bas` → `enrollment`

---

## 🐛 Common Errors

### "Tool not available"
- **Cause**: Tool not compiled or inactive
- **Fix**: Compile the `.bas` file first

### "Database connection error"
- **Cause**: Can't acquire DB lock
- **Fix**: Check database health

### "Timeout"
- **Cause**: Operation took >10 seconds
- **Fix**: Check database performance

---

## 💡 Pro Tips

1. **Verify additions**: Use `LIST_TOOLS` after adding tools
2. **Clean up**: Remove unused tools to improve LLM performance
3. **Session-specific**: Tools don't carry over to other sessions
4. **Backward compatible**: Legacy `current_tool` still works

---

## 📚 More Information

See `TOOL_MANAGEMENT.md` for comprehensive documentation including:
- Complete API reference
- Security details
- Performance optimization
- Testing strategies
- Troubleshooting guide

---

## 🔗 Related Files

- **Example Script**: `examples/tool_management_example.bas`
- **Implementation**: `src/basic/keywords/add_tool.rs`
- **Schema**: `migrations/6.0.3.sql`
- **Models**: `src/shared/models.rs`

---

## 📞 Support

For issues or questions:
1. Check the full documentation in `TOOL_MANAGEMENT.md`
2. Review the example script in `examples/`
3. Check database with: `SELECT * FROM session_tool_associations WHERE session_id = 'your-id';`
