PARAM resource_path AS STRING
PARAM review_period AS INTEGER DEFAULT 90

# Get current permissions
current_perms = CALL "/files/permissions", resource_path

# Get access logs
access_logs = CALL "/security/audit/logs", {
  "resource": resource_path,
  "action": "access",
  "timeframe": NOW() - DAYS(review_period)
}

# Identify inactive users with access
inactive_users = []
FOR EACH user IN current_perms
  # Check if user has accessed in review period
  user_logs = FILTER access_logs WHERE user_id = user.id
  
  IF LEN(user_logs) = 0 THEN
    APPEND inactive_users, {
      "user_id": user.id,
      "access_level": user.access_level,
      "last_access": CALL "/security/audit/logs", {
        "resource": resource_path,
        "action": "access",
        "user_id": user.id,
        "limit": 1
      }
    }
  END IF
NEXT

# Generate review report
review_report = {
  "resource": resource_path,
  "review_date": NOW(),
  "total_users_with_access": LEN(current_perms),
  "inactive_users": inactive_users,
  "recommendations": []
}

# Add recommendations
IF LEN(inactive_users) > 0 THEN
  review_report.recommendations.APPEND("Remove access for " + LEN(inactive_users) + " inactive users")
END IF

excessive_admins = FILTER current_perms WHERE access_level = "admin"
IF LEN(excessive_admins) > 3 THEN
  review_report.recommendations.APPEND("Reduce number of admin users (currently " + LEN(excessive_admins) + ")")
END IF

# Save review report
report_file = ".gbdata/security/access_reviews/" + REPLACE(resource_path, "/", "_") + "_" + FORMAT_DATE(NOW(), "Ymd") + ".json"
CALL "/files/save", report_file, review_report

# Notify security team
CALL "/comm/email/send", "security-team",
  "Access Review Report: " + resource_path,
  "A new access review report has been generated for " + resource_path + ".",
  [report_file]

RETURN review_report
