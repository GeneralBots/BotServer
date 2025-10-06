PARAM sender AS STRING
PARAM subject AS STRING
PARAM body AS STRING

# Get history for this sender
history = CALL "/storage/json", ".gbdata/communication_logs", "from = '${sender}' OR to = '${sender}' ORDER BY timestamp DESC LIMIT 10"

# Check if this is a known customer
customer = CALL "/crm/customers/get", sender

# Analyze email content
urgency = CALL "/ai/analyze/text", body, "urgency"
intent = CALL "/ai/analyze/text", body, "intent"
sentiment = CALL "/ai/analyze/text", body, "sentiment"

# Determine if auto-reply needed
should_auto_reply = FALSE

IF urgency.score > 0.8 THEN
  should_auto_reply = TRUE
END IF

IF customer IS NOT NULL AND customer.tier = "premium" THEN
  should_auto_reply = TRUE
END IF

IF intent.category = "support_request" THEN
  # Create support ticket
  ticket_id = CALL "/crm/tickets/create", {
    "customer": sender,
    "subject": subject,
    "description": body,
    "priority": urgency.score > 0.7 ? "High" : "Normal"
  }
  
  should_auto_reply = TRUE
  
  # Notify support team
  CALL "/comm/notifications/send", "support-team", 
    "New Support Ticket: " + subject,
    "A new support ticket has been created from an email by " + sender
END IF

IF should_auto_reply THEN
  reply_template = intent.category = "support_request" ? "support_acknowledgment" : "general_acknowledgment"
  
  reply_text = REWRITE "Based on this email: ${body}
  And this sender history: ${history}
  Generate a personalized auto-reply message using the ${reply_template} style.
  Include appropriate next steps and expected timeframe for response."
  
  CALL "/comm/email/send", "${user}", sender, "Re: " + subject, reply_text
  
  CALL "/storage/save", ".gbdata/auto_replies", {
    "to": sender,
    "subject": "Re: " + subject,
    "timestamp": NOW()
  }
END IF
