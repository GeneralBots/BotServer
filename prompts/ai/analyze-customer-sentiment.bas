PARAM customer_id AS STRING
PARAM time_period AS INTEGER DEFAULT 30

# Gather customer communications
emails = CALL "/storage/json", ".gbdata/communication_logs", 
  "to = '${customer_id}' OR from = '${customer_id}' AND timestamp > NOW() - DAYS(${time_period})"

support_tickets = CALL "/crm/tickets/list", {
  "customer_id": customer_id,
  "created_after": NOW() - DAYS(time_period)
}

meeting_notes = CALL "/crm/meetings/list", {
  "customer_id": customer_id,
  "date_after": NOW() - DAYS(time_period)
}

# Combine all text for analysis
all_text = ""
FOR EACH email IN emails
  all_text = all_text + email.subject + " " + email.body + " "
NEXT

FOR EACH ticket IN support_tickets
  all_text = all_text + ticket.description + " " + ticket.resolution + " "
NEXT

FOR EACH meeting IN meeting_notes
  all_text = all_text + meeting.notes + " "
NEXT

# Analyze sentiment
sentiment = CALL "/ai/analyze/text", all_text, "sentiment"

# Generate insights
insights = CALL "/ai/analyze/text", all_text, "key_topics"

RETURN {
  "customer_id": customer_id,
  "time_period": time_period + " days",
  "sentiment_score": sentiment.score,
  "sentiment_label": sentiment.label,
  "key_topics": insights.topics,
  "recommendations": insights.recommendations
}
