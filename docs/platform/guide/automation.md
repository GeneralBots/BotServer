# Automation System Documentation

## Overview

The automation system allows you to execute scripts automatically based on triggers like database changes or scheduled times.

## Database Configuration

### system_automations Table Structure

To create an automation, insert a record into the `system_automations` table:

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Unique identifier (auto-generated) |
| name | TEXT | Human-readable name |
| kind | INTEGER | Trigger type (see below) |
| target | TEXT | Target table name (for table triggers) |
| param | TEXT | Script filename or path |
| schedule | TEXT | Cron pattern (for scheduled triggers) |
| is_active | BOOLEAN | Whether automation is enabled |
| last_triggered | TIMESTAMP | Last execution time |

### Trigger Types (kind field)

- `0` - TableInsert (triggers on new rows)
- `1` - TableUpdate (triggers on row updates)
- `2` - TableDelete (triggers on row deletions)
- `3` - Scheduled (triggers on cron schedule)

## Configuration Examples

### 1. Scheduled Automation (Daily at 2:30 AM)
```sql
INSERT INTO system_automations (name, kind, target, param, schedule, is_active)
VALUES ('Daily Resume Update', 3, NULL, 'daily_resume.js', '30 2 * * *', true);
```

### 2. Table Change Automation
```sql
-- Trigger when new documents are added to documents table
INSERT INTO system_automations (name, kind, target, param, schedule, is_active)
VALUES ('Process New Documents', 0, 'documents', 'process_document.js', NULL, true);
```

## Cron Pattern Format

Use standard cron syntax: `minute hour day month weekday`

Examples:
- `0 9 * * *` - Daily at 9:00 AM
- `30 14 * * 1-5` - Weekdays at 2:30 PM
- `0 0 1 * *` - First day of every month at midnight

## Sample Script

```BASIC
    let text = GET "default.gbdrive/default.pdf"

    let resume = LLM "Build table resume with deadlines, dates and actions: " + text

    SET BOT MEMORY "resume", resume
```

## Script Capabilities

### Available Commands
- `GET "path"` - Read files from storage
- `LLM "prompt"` - Query language model with prompts
- `SET BOT MEMORY "key", value` - Store data in bot memory
- Database operations (query, insert, update)
- HTTP requests to external APIs

## Best Practices

1. **Keep scripts focused** - Each script should do one thing well
2. **Handle errors gracefully** - Use try/catch blocks
3. **Log important actions** - Use console.log for debugging
4. **Test thoroughly** - Verify scripts work before automating
5. **Monitor execution** - Check logs for any automation errors

## Monitoring

Check application logs to monitor automation execution:
```bash
# Look for automation-related messages
grep "Automation\|Script executed" application.log
```

The system will automatically update `last_triggered` timestamps and log any errors encountered during execution.
