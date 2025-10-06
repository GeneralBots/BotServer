PARAM name AS STRING
PARAM members AS ARRAY
PARAM description AS STRING OPTIONAL
PARAM team_type AS STRING DEFAULT "project"

# Create the group
group_id = CALL "/groups/create", {
  "name": name,
  "description": description,
  "type": team_type
}

# Add members
FOR EACH member IN members
  CALL "/groups/members/add", group_id, member
NEXT

# Create standard workspace structure
CALL "/files/createFolder", ".gbdrive/Workspaces/" + name + "/Documents"
CALL "/files/createFolder", ".gbdrive/Workspaces/" + name + "/Meetings"
CALL "/files/createFolder", ".gbdrive/Workspaces/" + name + "/Resources"

# Create default workspace components
IF team_type = "project" THEN
  # Create project board
  board_id = CALL "/tasks/create", {
    "title": name + " Project Board",
    "description": "Task board for " + name,
    "type": "project_board"
  }
  
  # Create standard task lanes
  lanes = ["Backlog", "To Do", "In Progress", "Review", "Done"]
  FOR EACH lane IN lanes
    CALL "/tasks/lanes/create", board_id, lane
  NEXT
  
  # Link group to project board
  CALL "/groups/settings", group_id, "project_board", board_id
END IF

# Set up communication channel
channel_id = CALL "/conversations/create", {
  "name": name,
  "description": description,
  "type": "group_chat"
}

# Add all members to channel
FOR EACH member IN members
  CALL "/conversations/members/add", channel_id, member
NEXT

# Link group to channel
CALL "/groups/settings", group_id, "conversation", channel_id

# Create welcome message
welcome_msg = REWRITE "Create a welcome message for a new workspace called ${name} with purpose: ${description}"

CALL "/conversations/messages/send", channel_id, {
  "text": welcome_msg,
  "pinned": TRUE
}

# Notify members
FOR EACH member IN members
  CALL "/comm/notifications/send", member, 
    "You've been added to " + name,
    "You have been added to the new workspace: " + name
NEXT

RETURN {
  "group_id": group_id,
  "channel_id": channel_id,
  "workspace_location": ".gbdrive/Workspaces/" + name
}
