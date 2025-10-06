PARAM from AS STRING
PARAM to AS STRING
PARAM subject AS STRING
PARAM body AS STRING
PARAM attachments AS ARRAY

# Track in communication history
CALL "/storage/save", ".gbdata/communication_logs", {
  "from": from,
  "to": to,
  "subject": subject,
  "timestamp": NOW(),
  "type": "email"
}

# Send actual email
CALL "/comm/email/send", from, to, subject, body, attachments

# If WITH HISTORY flag present, include prior communication 
IF WITH_HISTORY THEN
  prevComms = CALL "/storage/json", ".gbdata/communication_logs", "to = '" + to + "' ORDER BY timestamp DESC LIMIT 5"
  APPEND body WITH FORMAT_HISTORY(prevComms)
END IF
