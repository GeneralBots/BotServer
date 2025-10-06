PARAM period AS STRING DEFAULT "month"
PARAM team_id AS STRING OPTIONAL

# Determine date range
IF period = "week" THEN
  start_date = NOW() - DAYS(7)
ELSEIF period = "month" THEN
  start_date = NOW() - DAYS(30)
ELSEIF period = "quarter" THEN
  start_date = NOW() - DAYS(90)
ELSEIF period = "year" THEN
  start_date = NOW() - DAYS(365)
ELSE
  RETURN "Invalid period specified. Use 'week', 'month', 'quarter', or 'year'."
END IF

# Construct team filter
team_filter = ""
IF team_id IS NOT NULL THEN
  team_filter = " AND team_id = '" + team_id + "'"
END IF

# Get sales data
opportunities = QUERY "SELECT * FROM Opportunities WHERE close_date >= '${start_date}'" + team_filter
closed_won = QUERY "SELECT * FROM Opportunities WHERE status = 'Won' AND close_date >= '${start_date}'" + team_filter
closed_lost = QUERY "SELECT * FROM Opportunities WHERE status = 'Lost' AND close_date >= '${start_date}'" + team_filter

# Calculate metrics
total_value = 0
FOR EACH opp IN closed_won
  total_value = total_value + opp.value
NEXT

win_rate = LEN(closed_won) / (LEN(closed_won) + LEN(closed_lost)) * 100

# Get performance by rep
sales_reps = QUERY "SELECT owner_id, COUNT(*) as deals, SUM(value) as total_value FROM Opportunities WHERE status = 'Won' AND close_date >= '${start_date}'" + team_filter + " GROUP BY owner_id"

# Generate report
report = CALL "/analytics/reports/generate", {
  "title": "Sales Performance Report - " + UPPER(period),
  "date_range": "From " + FORMAT_DATE(start_date) + " to " + FORMAT_DATE(NOW()),
  "metrics": {
    "total_opportunities": LEN(opportunities),
    "won_opportunities": LEN(closed_won),
    "lost_opportunities": LEN(closed_lost),
    "win_rate": win_rate,
    "total_value": total_value
  },
  "rep_performance": sales_reps,
  "charts": [
    {
      "type": "bar",
      "title": "Won vs Lost Opportunities",
      "data": {"Won": LEN(closed_won), "Lost": LEN(closed_lost)}
    },
    {
      "type": "line",
      "title": "Sales Trend",
      "data": QUERY "SELECT DATE_FORMAT(close_date, '%Y-%m-%d') as date, COUNT(*) as count, SUM(value) as value FROM Opportunities WHERE status = 'Won' AND close_date >= '${start_date}'" + team_filter + " GROUP BY DATE_FORMAT(close_date, '%Y-%m-%d')"
    }
  ]
}

# Save report
report_file = ".gbdrive/Reports/Sales/sales_performance_" + period + "_" + FORMAT_DATE(NOW(), "Ymd") + ".pdf"
CALL "/files/save", report_file, report

# Share report
IF team_id IS NOT NULL THEN
  CALL "/files/shareFolder", report_file, team_id
  
  # Notify team manager
  manager = QUERY "SELECT manager_id FROM Teams WHERE id = '${team_id}'"
  IF LEN(manager) > 0 THEN
    CALL "/comm/email/send", manager[0],
      "Sales Performance Report - " + UPPER(period),
      "The latest sales performance report for your team is now available.",
      [report_file]
  END IF
END IF

RETURN "Sales performance report generated: " + report_file
