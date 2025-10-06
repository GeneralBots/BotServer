PARAM attendees AS ARRAY
PARAM topic AS STRING
PARAM duration AS INTEGER
PARAM preferred_date AS DATE OPTIONAL

# Find available time for all attendees
IF preferred_date IS NULL THEN
  available_slots = CALL "/calendar/availability/check", attendees, NOW(), NOW() + DAYS(7), duration
ELSE
  available_slots = CALL "/calendar/availability/check", attendees, preferred_date, preferred_date + DAYS(1), duration
END IF

IF LEN(available_slots) = 0 THEN
  RETURN "No available time slots found for all attendees."
END IF

# Create meeting description
description = REWRITE "Generate a concise meeting description for topic: ${topic}"

# Schedule the meeting
event_id = CALL "/calendar/events/create", {
  "subject": topic,
  "description": description,
  "start_time": available_slots[0].start,
  "end_time": available_slots[0].end,
  "attendees": attendees,
  "location": "Virtual Meeting"
}

# Notify attendees
FOR EACH person IN attendees
  CALL "/comm/notifications/send", person, "Meeting Scheduled: " + topic, 
    "You have been invited to a meeting on " + FORMAT_DATE(available_slots[0].start)
NEXT

RETURN "Meeting scheduled for " + FORMAT_DATE(available_slots[0].start)
