PARAM components AS ARRAY OPTIONAL
PARAM notify AS BOOLEAN DEFAULT TRUE

# Check all components by default
IF components IS NULL THEN
  components = ["storage", "api", "database", "integrations", "security"]
END IF

status_report = {}

FOR EACH component IN components
  status = CALL "/health/detailed", component
  status_report[component] = status
NEXT

# Calculate overall health score
total_score = 0
FOR EACH component IN components
  total_score = total_score + status_report[component].health_score
NEXT

overall_health = total_score / LEN(components)
status_report["overall_health"] = overall_health
status_report["timestamp"] = NOW()

# Save status report
CALL "/storage/save", ".gbdata/health/status_" + FORMAT_DATE(NOW(), "Ymd_His") + ".json", status_report

# Check for critical issues
critical_issues = []
FOR EACH component IN components
  IF status_report[component].health_score < 0.7 THEN
    APPEND critical_issues, {
      "component": component,
      "score": status_report[component].health_score,
      "issues": status_report[component].issues
    }
  END IF
NEXT

# Notify if critical issues found
IF LEN(critical_issues) > 0 AND notify THEN
  issue_summary = "Critical system health issues detected:\n\n"
  FOR EACH issue IN critical_issues
    issue_summary = issue_summary + "- " + issue.component + " (Score: " + issue.score + ")\n"
    FOR EACH detail IN issue.issues
      issue_summary = issue_summary + "  * " + detail + "\n"
    NEXT
    issue_summary = issue_summary + "\n"
  NEXT
  
  CALL "/comm/notifications/send", "admin-team", 
    "ALERT: System Health Issues Detected",
    issue_summary,
    "high"
END IF

RETURN status_report
