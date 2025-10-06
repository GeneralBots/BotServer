PARAM opportunity_id AS STRING
PARAM status AS STRING
PARAM notes AS STRING OPTIONAL
PARAM next_steps AS STRING OPTIONAL

# Get current opportunity data
opp_data = QUERY "SELECT * FROM Opportunities WHERE Id = '${opportunity_id}'"

IF LEN(opp_data) = 0 THEN
  RETURN "Opportunity not found."
END IF

# Update opportunity status
CALL "/crm/opportunities/update", opportunity_id, {
  "status": status,
  "last_updated": NOW(),
  "updated_by": "${user}"
}

# Add activity note if provided
IF notes IS NOT NULL THEN
  CALL "/crm/activities/create", opportunity_id, "note", {
    "description": notes,
    "date": NOW()
  }
END IF

# Set follow-up task if next steps provided
IF next_steps IS NOT NULL THEN
  CALL "/tasks/create", {
    "title": "Follow up: " + opp_data[0].company,
    "description": next_steps,
    "due_date": NOW() + DAYS(3),
    "assigned_to": "${user}",
    "related_to": opportunity_id
  }
END IF

# Notify sales manager of major status changes
IF status = "Won" OR status = "Lost" THEN
  manager = QUERY "SELECT Manager FROM Users WHERE Username = '${user}'"
  CALL "/comm/notifications/send", manager[0], 
    "Opportunity " + status + ": " + opp_data[0].company,
    "The opportunity with " + opp_data[0].company + " has been marked as " + status + " by ${user}."
END IF

RETURN "Opportunity status updated to " + status
