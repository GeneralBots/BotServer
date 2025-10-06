PARAM meeting_id AS STRING
PARAM action AS STRING DEFAULT "join"

IF action = "join" THEN
  # Get meeting details
  meeting = CALL "/calendar/events/get", meeting_id
  
  # Join the meeting
  CALL "/conversations/calls/join", meeting.conference_link
  
  # Set up recording
  CALL "/conversations/recording/start", meeting_id
  
  # Create meeting notes document
  notes_doc = CALL "/files/create", 
    ".gbdrive/Meetings/" + meeting.subject + "_" + FORMAT_DATE(NOW(), "Ymd") + ".md",
    "# Meeting Notes: " + meeting.subject + "\n\n" +
    "Date: " + FORMAT_DATE(meeting.start) + "\n\n" +
    "Participants: \n" +
    "- " + JOIN(meeting.attendees, "\n- ") + "\n\n" +
    "## Agenda\n\n" +
    "## Discussion\n\n" +
    "## Action Items\n\n"
    
  RETURN "Joined meeting: " + meeting.subject

ELSEIF action = "summarize" THEN
  # Get recording transcript
  transcript = CALL "/conversations/recording/transcript", meeting_id
  
  # Generate meeting summary
  summary = CALL "/ai/summarize", transcript, {
    "format": "meeting_notes",
    "sections": ["key_points", "decisions", "action_items"]
  }
  
  # Update meeting notes
  meeting = CALL "/calendar/events/get", meeting_id
  notes_path = ".gbdrive/Meetings/" + meeting.subject + "_" + FORMAT_DATE(NOW(), "Ymd") + ".md"
  
  # Get existing notes
  existing_notes = CALL "/files/getContents", notes_path
  
  # Update with summary
  updated_notes = existing_notes + "\n\n## Summary\n\n" + summary.key_points + 
    "\n\n## Decisions\n\n" + summary.decisions +
    "\n\n## Action Items\n\n" + summary.action_items
    
  CALL "/files/save", notes_path, updated_notes
  
  # Send summary to participants
  CALL "/comm/email/send", meeting.attendees,
    "Meeting Summary: " + meeting.subject,
    "Please find attached the summary of our recent meeting.",
    [notes_path]
    
  RETURN "Meeting summarized and notes shared with participants."

ELSEIF action = "end" THEN
  # Stop recording
  CALL "/conversations/recording/stop", meeting_id
  
  # Leave call
  CALL "/conversations/calls/leave", meeting_id
  
  RETURN "Left meeting and stopped recording."
END IF
