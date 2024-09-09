REM SET SCHEDULER "1 * * * * "

data = FIND "reminder.csv", "when=" + hour

if (data) THEN
    TALK TO admin, data.subject
end if 