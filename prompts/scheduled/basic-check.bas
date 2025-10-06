SET SCHEDULE every 1 hour

# Check emails
unread_emails = CALL "/comm/email/list", {
  "status": "unread", 
  "folder": "inbox", 
  "max_age": "24h"
}

# Check calendar
upcoming_events = CALL "/calendar/events/list", {
  "start": NOW(),
  "end": NOW() + HOURS(24)
}

# Check tasks
due_tasks = CALL "/tasks/list", {
  "status": "open",
  "due_before": NOW() + HOURS(24)
}

# Check important documents
new_documents = CALL "/files/recent", {
  "folders": [".gbdrive/papers", ".gbdrive/Proposals"],
  "since": NOW() - HOURS(24)
}

# Prepare notification message
notification = "Daily Update:\n"

IF LEN(unread_emails) > 0 THEN
  notification = notification + "- You have " + LEN(unread_emails) + " unread emails\n"
END IF

IF LEN(upcoming_events) > 0 THEN
  notification = notification + "- You have " + LEN(upcoming_events) + " upcoming meetings in the next 24 hours\n"
  notification = notification + "  Next: " + upcoming_events[0].subject + " at " + FORMAT_TIME(upcoming_events[0].start) + "\n"
END IF

IF LEN(due_tasks) > 0 THEN
  notification = notification + "- You have " + LEN(due_tasks) + " tasks due in the next 24 hours\n"
END IF

IF LEN(new_documents) > 0 THEN
  notification = notification + "- " + LEN(new_documents) + " new documents have been added to your monitored folders\n"
END IF

# Send notification
IF LEN(notification) > "Daily Update:\n" THEN
  CALL "/comm/notifications/send", "${user}", "Daily Status Update", notification
END IF
